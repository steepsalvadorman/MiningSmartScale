/// Tests de integración de WeighFlow IoT — sin hardware físico.
/// Etapas cubiertas: parser, sealer (HMAC), API HTTP, export CSV.
/// Inyectan bytes directamente en el pipeline (parser → sealer → API)
/// para verificar el comportamiento completo del sistema.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tokio::sync::{broadcast, mpsc, Mutex};
use tower::ServiceExt;
use weighflow::{api::AppState, events::WeighEvent, parser, sealer};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Construye bytes de N tramas STX/ETX con el peso dado.
fn stx_etx_frames(n: usize, weight: f64, unit: &str, stable: bool) -> Vec<u8> {
    let status = if stable { "S" } else { "US" };
    let mut buf = Vec::new();
    for _ in 0..n {
        buf.push(0x02);
        buf.extend_from_slice(
            format!("{status}   {weight:.3} {unit}").as_bytes(),
        );
        buf.push(0x03);
        buf.push(0x41); // checksum ficticio
    }
    buf
}

/// Construye bytes de N líneas CR/LF con el peso dado.
fn crlf_frames(n: usize, weight: f64, unit: &str, stable: bool) -> Vec<u8> {
    let status = if stable { "S" } else { "US" };
    let line = format!("{status}   {weight:.3} {unit}\r\n");
    line.repeat(n).into_bytes()
}

/// Corre el pipeline parser → sealer y devuelve los SignedEvents producidos.
/// Envía `detection` para que el detector identifique el protocolo,
/// luego envía `payload` para que el parser emita eventos reales.
async fn run_pipeline(
    detection: Vec<u8>,
    payload: Vec<u8>,
    hmac_key: &[u8],
) -> Vec<weighflow::events::SignedEvent> {
    let (raw_tx, raw_rx) = broadcast::channel::<Vec<u8>>(64);
    let (event_tx, event_rx) = mpsc::channel::<WeighEvent>(64);
    let events_buf: Arc<Mutex<VecDeque<weighflow::events::SignedEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));
    let (live_tx, _) = broadcast::channel(64);

    tokio::spawn(parser::run(raw_rx, event_tx));
    tokio::spawn(sealer::run(
        event_rx,
        events_buf.clone(),
        live_tx,
        1000,
        hmac_key.to_vec(),
    ));

    // Fase 1: bytes de detección (el detector necesita MIN_BOUNDARIES=3)
    raw_tx.send(detection).unwrap();

    // Dar tiempo al detector para que procese y establezca el protocolo
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Fase 2: bytes reales que queremos parsear
    raw_tx.send(payload).unwrap();

    // Esperar a que el pipeline procese
    tokio::time::sleep(Duration::from_millis(50)).await;

    events_buf.lock().await.iter().cloned().collect()
}

// ── Tests: Parser CR/LF ───────────────────────────────────────────────────────

#[tokio::test]
async fn crlf_extrae_peso_correcto() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 24.550, "kg", true);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert!(!events.is_empty(), "Debe producir al menos un evento");
    let e = &events[0];
    assert!((e.value - 24.550).abs() < 0.01, "Peso incorrecto: {}", e.value);
    assert_eq!(e.unit, "kg");
    assert!(e.stable, "Debe ser estable");
}

#[tokio::test]
async fn crlf_detecta_lectura_inestable() {
    let detection = crlf_frames(3, 0.0, "kg", false);
    let payload = crlf_frames(1, 18.200, "kg", false);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert!(!events.is_empty());
    assert!(!events[0].stable, "Debe ser EN MOVIMIENTO");
}

#[tokio::test]
async fn crlf_multiples_lecturas_en_payload() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(5, 10.000, "kg", true);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert_eq!(events.len(), 5, "Debe producir 5 eventos, produjo {}", events.len());
}

#[tokio::test]
async fn crlf_peso_cero_balanza_vacia() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 0.000, "kg", true);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert!(!events.is_empty());
    assert!(events[0].value.abs() < 0.001, "Peso debería ser ~0");
}

// ── Tests: Parser STX/ETX ─────────────────────────────────────────────────────

#[tokio::test]
async fn stx_etx_extrae_peso_correcto() {
    let detection = stx_etx_frames(3, 0.0, "kg", true);
    let payload = stx_etx_frames(1, 52.750, "kg", true);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert!(!events.is_empty());
    assert!((events[0].value - 52.750).abs() < 0.01, "Peso: {}", events[0].value);
    assert_eq!(events[0].unit, "kg");
}

#[tokio::test]
async fn stx_etx_detecta_inestable() {
    let detection = stx_etx_frames(3, 0.0, "kg", false);
    let payload = stx_etx_frames(1, 5.100, "kg", false);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert!(!events.is_empty());
    assert!(!events[0].stable);
}

#[tokio::test]
async fn stx_etx_multiples_eventos() {
    let detection = stx_etx_frames(3, 0.0, "kg", true);
    let payload = stx_etx_frames(4, 33.000, "kg", true);

    let events = run_pipeline(detection, payload, b"test-key").await;

    assert_eq!(events.len(), 4);
}

// ── Tests: HMAC / Sellado criptográfico ──────────────────────────────────────

#[tokio::test]
async fn evento_firmado_pasa_verificacion() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 100.000, "kg", true);
    let key = b"clave-secreta-produccion";

    let events = run_pipeline(detection, payload, key).await;

    assert!(!events.is_empty());
    let valid = weighflow::sealer::verify(&events[0], key);
    assert!(valid, "El HMAC debe ser válido con la clave correcta");
}

#[tokio::test]
async fn evento_alterado_falla_verificacion() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 50.000, "kg", true);
    let key = b"clave-secreta";

    let mut events = run_pipeline(detection, payload, key).await;
    assert!(!events.is_empty());

    // Simular alteración del peso después del sellado
    events[0].value = 9999.999;

    let valid = weighflow::sealer::verify(&events[0], key);
    assert!(!valid, "HMAC debe fallar cuando el valor fue alterado");
}

#[tokio::test]
async fn clave_incorrecta_falla_verificacion() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 25.000, "kg", true);

    let events = run_pipeline(detection, payload, b"clave-correcta").await;
    assert!(!events.is_empty());

    let valid = weighflow::sealer::verify(&events[0], b"clave-incorrecta");
    assert!(!valid, "HMAC debe fallar con clave distinta");
}

#[tokio::test]
async fn hmac_tiene_64_caracteres_hex() {
    let detection = crlf_frames(3, 0.0, "kg", true);
    let payload = crlf_frames(1, 1.0, "kg", true);

    let events = run_pipeline(detection, payload, b"key").await;
    assert!(!events.is_empty());
    assert_eq!(events[0].hmac.len(), 64, "HMAC-SHA256 debe tener 64 chars hex");
}

// ── Tests: API HTTP ───────────────────────────────────────────────────────────

#[tokio::test]
async fn api_status_sin_eventos_devuelve_desconectado() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["connected"], false);
    assert_eq!(json["event_count"], 0);
    assert!(json["last_event"].is_null());
}

#[tokio::test]
async fn api_events_inicialmente_vacio() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn api_events_devuelve_eventos_insertados() {
    use weighflow::events::SignedEvent;

    let state = Arc::new(AppState::new(100));

    // Insertar evento directamente en el buffer
    let event = SignedEvent {
        id: 1,
        value: 24.550,
        unit: "kg".to_string(),
        stable: true,
        timestamp_ms: 1700000000000,
        hmac: "a".repeat(64),
    };
    state.events.lock().await.push_back(event);

    let app = weighflow::api::create_router(state);
    let response = app
        .oneshot(Request::builder().uri("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();

    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["value"], 24.550);
    assert_eq!(arr[0]["unit"], "kg");
    assert_eq!(arr[0]["stable"], true);
}

// ── Tests: Export CSV ─────────────────────────────────────────────────────────

#[tokio::test]
async fn export_csv_devuelve_content_type_correcto() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/export/csv").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/csv"), "Content-Type debe ser text/csv, es: {}", ct);
}

#[tokio::test]
async fn export_csv_incluye_encabezado_y_datos() {
    use weighflow::events::SignedEvent;

    let state = Arc::new(AppState::new(100));

    let event = SignedEvent {
        id: 7,
        value: 99.123,
        unit: "kg".to_string(),
        stable: true,
        timestamp_ms: 1700000000000,
        hmac: "c".repeat(64),
    };
    state.events.lock().await.push_back(event);

    let app = weighflow::api::create_router(state);
    let response = app
        .oneshot(Request::builder().uri("/export/csv").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let csv = std::str::from_utf8(&body).unwrap();

    assert!(csv.contains("id,fecha_hora,"), "Falta encabezado CSV");
    assert!(csv.contains("7,"), "Falta fila con id=7");
    assert!(csv.contains("99.123000"), "Falta valor con 6 decimales");
    assert!(csv.contains(",kg,"), "Falta unidad kg");
}

#[tokio::test]
async fn export_csv_vacio_solo_tiene_encabezado() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/export/csv").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let csv = std::str::from_utf8(&body).unwrap();
    let lines: Vec<&str> = csv.lines().collect();

    assert_eq!(lines.len(), 1, "Sin datos debe haber solo el encabezado");
}

#[tokio::test]
async fn api_status_con_evento_reciente_marca_conectado() {
    use std::time::{SystemTime, UNIX_EPOCH};
    use weighflow::events::SignedEvent;

    let state = Arc::new(AppState::new(100));

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let event = SignedEvent {
        id: 1,
        value: 50.0,
        unit: "kg".to_string(),
        stable: true,
        timestamp_ms: now_ms, // evento justo ahora
        hmac: "b".repeat(64),
    };
    state.events.lock().await.push_back(event);

    let app = weighflow::api::create_router(state);
    let response = app
        .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["connected"], true, "Evento reciente debe marcar connected=true");
    assert_eq!(json["event_count"], 1);
}

// ── Tests: Dashboard embebido ─────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_sirve_html_con_content_type_correcto() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/html"), "Content-Type debe ser text/html, es: {}", ct);
}

#[tokio::test]
async fn dashboard_html_referencia_websocket_live() {
    let state = Arc::new(AppState::new(100));
    let app = weighflow::api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = std::str::from_utf8(&body).unwrap();

    assert!(html.contains("WeighFlow"), "HTML debe contener el nombre de la aplicación");
    assert!(html.contains("/live"), "HTML debe referenciar el endpoint WebSocket /live");
    assert!(html.contains("/export/csv"), "HTML debe referenciar la descarga CSV");
    assert!(html.contains("/events"), "HTML debe referenciar el endpoint de eventos");
}
