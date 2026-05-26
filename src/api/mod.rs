pub mod ws;

use axum::{
    body::Body,
    extract::State,
    http::{header, Response, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::events::SignedEvent;

/// Estado compartido entre todos los handlers HTTP/WS.
/// Se pasa como `Arc<AppState>` para que el clone sea barato.
#[derive(Clone)]
pub struct AppState {
    pub events: Arc<Mutex<VecDeque<SignedEvent>>>,
    pub live_tx: broadcast::Sender<SignedEvent>,
    pub max_events: usize,
}

impl AppState {
    pub fn new(max_events: usize) -> Self {
        let (live_tx, _) = broadcast::channel(256);
        Self {
            events: Arc::new(Mutex::new(VecDeque::new())),
            live_tx,
            max_events,
        }
    }
}

/// Construye el Router de Axum. Separado de `serve` para permitir tests sin TCP.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/events", get(get_events))
        .route("/status", get(get_status))
        .route("/export/csv", get(export_csv))
        .route("/live", get(ws::handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn serve(state: Arc<AppState>, port: u16) {
    let router = create_router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    info!("─────────────────────────────────────────────────");
    info!("  Dashboard:  http://localhost:{}/", port);
    info!("  GET  http://localhost:{}/events      — historial (JSON)", port);
    info!("  GET  http://localhost:{}/status      — estado del sistema", port);
    info!("  GET  http://localhost:{}/export/csv  — descargar CSV del turno", port);
    info!("  WS   ws://localhost:{}/live          — stream en tiempo real", port);
    info!("─────────────────────────────────────────────────");

    axum::serve(listener, router).await.unwrap();
}

// ── Handlers ──────────────────────────────────────────────────────────────────

static DASHBOARD_HTML: &str = include_str!("../../static/dashboard.html");

async fn dashboard() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(DASHBOARD_HTML))
        .unwrap()
}

async fn export_csv(State(state): State<Arc<AppState>>) -> Response<Body> {
    let buf = state.events.lock().await;
    let csv = crate::export::to_csv(&buf);
    let filename = format!(
        "attachment; filename=\"pesajes_{}.csv\"",
        crate::export::today_str()
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, filename)
        .body(Body::from(csv))
        .unwrap()
}

async fn get_events(State(state): State<Arc<AppState>>) -> Json<Vec<SignedEvent>> {
    let buf = state.events.lock().await;
    Json(buf.iter().cloned().collect())
}

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    event_count: usize,
    last_event: Option<SignedEvent>,
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let buf = state.events.lock().await;
    let last = buf.back().cloned();

    let connected = last.as_ref().map_or(false, |e| {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms.saturating_sub(e.timestamp_ms) < 10_000
    });

    Json(StatusResponse {
        connected,
        event_count: buf.len(),
        last_event: last,
    })
}
