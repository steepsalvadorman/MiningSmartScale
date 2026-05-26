/// Pruebas de robustez en condiciones de campo — sin hardware físico.
///
/// Simulan situaciones reales en operaciones mineras:
/// - Ruido eléctrico antes/entre tramas RS-232
/// - Tramas partidas por latencia del bus serial
/// - Basura masiva (balanza recién encendida, cable suelto)
/// - Límite del buffer circular de eventos
/// - Monotonicidad de IDs bajo carga
/// - Unidades distintas (kg / lb)
/// - Pesos fuera del rango válido

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc, Mutex};
use weighflow::{events::WeighEvent, parser, sealer};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Genera una trama CRLF completa.
fn crlf(weight: f64, unit: &str, stable: bool) -> Vec<u8> {
    let st = if stable { "S" } else { "US" };
    format!("{st}   {weight:.3} {unit}\r\n").into_bytes()
}

/// Genera una trama STX/ETX completa.
fn stx_etx(weight: f64, unit: &str, stable: bool) -> Vec<u8> {
    let st = if stable { "S" } else { "US" };
    let mut v = vec![0x02u8];
    v.extend_from_slice(format!("{st}   {weight:.3} {unit}").as_bytes());
    v.push(0x03);
    v.push(0x41); // checksum ficticio
    v
}

/// 3 tramas CRLF para la fase de detección del detector.
fn det_crlf() -> Vec<u8> {
    (0..3).flat_map(|_| crlf(0.0, "kg", true)).collect()
}

/// 3 tramas STX/ETX para la fase de detección.
fn det_stx() -> Vec<u8> {
    (0..3).flat_map(|_| stx_etx(0.0, "kg", true)).collect()
}

/// Pipeline completo: parser + sealer.
/// Envía bytes de detección, espera que el detector decida,
/// luego envía cada chunk con una pausa entre ellos.
/// `max_events` controla el tamaño del buffer circular del sealer.
async fn pipeline(
    detection: Vec<u8>,
    chunks: Vec<Vec<u8>>,
    hmac_key: &[u8],
    max_events: usize,
) -> Vec<weighflow::events::SignedEvent> {
    let (raw_tx, raw_rx) = broadcast::channel::<Vec<u8>>(64);
    let (ev_tx, ev_rx) = mpsc::channel::<WeighEvent>(64);
    let buf = Arc::new(Mutex::new(VecDeque::new()));
    let (live_tx, _) = broadcast::channel(64);

    tokio::spawn(parser::run(raw_rx, ev_tx));
    tokio::spawn(sealer::run(
        ev_rx,
        buf.clone(),
        live_tx,
        max_events,
        hmac_key.to_vec(),
    ));

    raw_tx.send(detection).unwrap();
    tokio::time::sleep(Duration::from_millis(25)).await;

    for chunk in chunks {
        raw_tx.send(chunk).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    tokio::time::sleep(Duration::from_millis(80)).await;
    buf.lock().await.iter().cloned().collect()
}

// Atajo para un solo payload sin chunking.
async fn single(det: Vec<u8>, payload: Vec<u8>, key: &[u8]) -> Vec<weighflow::events::SignedEvent> {
    pipeline(det, vec![payload], key, 1000).await
}

// ── Prueba 1: bytes de ruido ANTES de una trama válida ────────────────────────

#[tokio::test]
async fn ruido_prefijo_no_impide_parseo() {
    // Bytes de ruido eléctrico (no ASCII imprimible) antes de la trama
    let mut payload = vec![0xFF, 0xFE, 0x00, 0xAA, 0xBB];
    payload.extend_from_slice(&crlf(42.0, "kg", true));

    let events = single(det_crlf(), payload, b"test").await;

    assert!(!events.is_empty(), "Debe extraer el evento a pesar del ruido prefijo");
    assert!(
        events.iter().any(|e| (e.value - 42.0).abs() < 0.01),
        "Debe encontrar el peso 42.000 kg"
    );
}

// ── Prueba 2: trama CRLF partida en dos envíos (latencia bus serial) ──────────

#[tokio::test]
async fn frame_crlf_partido_en_dos_envios() {
    // "S   24.500 kg\r\n" se parte justo antes del delimitador
    let chunk1 = b"S   24.500 kg".to_vec();
    let chunk2 = b"\r\n".to_vec();

    let events = pipeline(det_crlf(), vec![chunk1, chunk2], b"test", 1000).await;

    assert!(!events.is_empty(), "Trama partida debe producir un evento");
    assert!(
        (events[0].value - 24.5).abs() < 0.01,
        "Peso incorrecto: {}",
        events[0].value
    );
}

// ── Prueba 3: ruido entre tramas válidas ──────────────────────────────────────

#[tokio::test]
async fn ruido_entre_frames_validos_produce_dos_eventos() {
    // Trama válida + basura no-ASCII + trama válida
    // parse_ascii_frame filtra bytes no-gráficos antes de extraer el número
    let mut payload = Vec::new();
    payload.extend_from_slice(&crlf(10.0, "kg", true));
    payload.extend_from_slice(&[0xFF, 0xFE]); // ruido sin \r\n — queda en el buffer
    payload.extend_from_slice(&crlf(20.0, "kg", true));

    let events = single(det_crlf(), payload, b"test").await;

    assert_eq!(events.len(), 2, "Debe producir 2 eventos: {:?}", events.iter().map(|e| e.value).collect::<Vec<_>>());
    assert!((events[0].value - 10.0).abs() < 0.01);
    assert!((events[1].value - 20.0).abs() < 0.01);
}

// ── Prueba 4: 1 KB de basura pura — no debe crashear, no debe producir eventos ─

#[tokio::test]
async fn basura_masiva_no_produce_eventos() {
    // Simula cable suelto o balanza encendiéndose: ráfaga de 0xFF sin \r\n
    let payload = vec![0xFF; 1024];

    let events = single(det_crlf(), payload, b"test").await;

    // Sin \r\n no hay línea completa; no deben producirse eventos
    assert!(
        events.is_empty(),
        "1 KB de 0xFF no debe producir eventos (produjo {})",
        events.len()
    );
}

// ── Prueba 5: línea ASCII sin número válido se descarta ───────────────────────

#[tokio::test]
async fn linea_sin_numero_valido_se_descarta() {
    let payload = b"S   ERROR OVERFLOW\r\n".to_vec();

    let events = single(det_crlf(), payload, b"test").await;

    assert!(
        events.is_empty(),
        "Línea sin número debe descartarse"
    );
}

// ── Prueba 6: peso ≥ 1 000 000 kg se descarta (fuera del rango razonable) ──────

#[tokio::test]
async fn peso_fuera_de_rango_se_descarta() {
    let payload = b"S   1500000.000 kg\r\n".to_vec();

    let events = single(det_crlf(), payload, b"test").await;

    assert!(
        events.is_empty(),
        "Peso >= 1_000_000 debe ser descartado como dato inválido"
    );
}

// ── Prueba 7: buffer circular descarta los eventos más antiguos ───────────────

#[tokio::test]
async fn ring_buffer_descarta_mas_antiguos() {
    // Enviamos 7 eventos con max_events = 3
    // Resultado esperado: sólo los últimos 3 (pesos 5, 6, 7)
    let payload: Vec<u8> = (1u32..=7)
        .flat_map(|w| crlf(w as f64, "kg", true))
        .collect();

    let events = pipeline(det_crlf(), vec![payload], b"test", 3).await;

    assert_eq!(
        events.len(),
        3,
        "Ring buffer de tamaño 3 debe conservar exactamente 3 eventos"
    );
    // VecDeque mantiene orden: más antiguo al frente, más nuevo al fondo
    assert!((events[0].value - 5.0).abs() < 0.01, "Evento [0] debe ser ~5.0, es {}", events[0].value);
    assert!((events[1].value - 6.0).abs() < 0.01, "Evento [1] debe ser ~6.0, es {}", events[1].value);
    assert!((events[2].value - 7.0).abs() < 0.01, "Evento [2] debe ser ~7.0, es {}", events[2].value);
}

// ── Prueba 8: IDs son estrictamente crecientes ────────────────────────────────

#[tokio::test]
async fn ids_son_monotonicamente_crecientes() {
    let payload: Vec<u8> = (1u32..=5)
        .flat_map(|w| crlf(w as f64, "kg", true))
        .collect();

    let events = single(det_crlf(), payload, b"test").await;

    assert_eq!(events.len(), 5);
    for w in events.windows(2) {
        assert!(
            w[0].id < w[1].id,
            "IDs deben ser estrictamente crecientes: {} → {}",
            w[0].id,
            w[1].id
        );
    }
}

// ── Prueba 9: unidad libras (lb) se detecta correctamente ─────────────────────

#[tokio::test]
async fn unidad_libras_se_detecta() {
    let detection: Vec<u8> = (0..3).flat_map(|_| b"S   10.000 lb\r\n".to_vec()).collect();
    let payload = b"S   55.250 lb\r\n".to_vec();

    let events = single(detection, payload, b"test").await;

    assert!(!events.is_empty(), "Debe detectar lectura en libras");
    assert_eq!(events[0].unit, "lb", "Unidad incorrecta: {}", events[0].unit);
    assert!((events[0].value - 55.25).abs() < 0.01);
}

// ── Prueba 10: trama STX/ETX partida en dos envíos ────────────────────────────

#[tokio::test]
async fn stx_etx_frame_partido_en_dos_envios() {
    // [STX] + "S   88.0" en chunk 1; "00 kg" + [ETX] + checksum en chunk 2
    let chunk1 = {
        let mut v = vec![0x02u8];
        v.extend_from_slice(b"S   88.0");
        v
    };
    let chunk2 = {
        let mut v = b"00 kg".to_vec();
        v.push(0x03);
        v.push(0x41);
        v
    };

    let events = pipeline(det_stx(), vec![chunk1, chunk2], b"test", 1000).await;

    assert!(!events.is_empty(), "STX/ETX partido en dos envíos debe producir un evento");
    assert!(
        (events[0].value - 88.0).abs() < 0.01,
        "Peso incorrecto: {}",
        events[0].value
    );
}
