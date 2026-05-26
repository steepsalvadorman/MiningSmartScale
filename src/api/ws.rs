use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::api::AppState;

pub async fn handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.live_tx.subscribe();
    info!("[WS] Cliente conectado — streaming en tiempo real");

    loop {
        match rx.recv().await {
            Ok(event) => {
                match serde_json::to_string(&event) {
                    Ok(json) => {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break; // cliente desconectado
                        }
                    }
                    Err(e) => warn!("[WS] Error serializando evento #{}: {}", event.id, e),
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                warn!("[WS] Cliente lento — se perdieron {} eventos", n);
            }
            Err(_) => break,
        }
    }

    info!("[WS] Cliente desconectado");
}
