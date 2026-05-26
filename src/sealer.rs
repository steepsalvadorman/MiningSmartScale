use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{debug, info, warn};

use crate::events::{SignedEvent, WeighEvent};

type HmacSha256 = Hmac<Sha256>;

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Task async que recibe WeighEvent del parser, los firma con HMAC-SHA256
/// y los distribuye al buffer circular y a los clientes WebSocket.
pub async fn run(
    mut rx: mpsc::Receiver<WeighEvent>,
    events: Arc<Mutex<VecDeque<SignedEvent>>>,
    live_tx: broadcast::Sender<SignedEvent>,
    max_events: usize,
    hmac_key: Vec<u8>,
) {
    info!("[SEALER] Iniciado — firmando eventos con HMAC-SHA256");

    while let Some(event) = rx.recv().await {
        let id = EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);

        let timestamp_ms = event
            .timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let unit_str = event.unit.to_string();
        let hmac = sign(id, event.value, timestamp_ms, &unit_str, &hmac_key);

        let signed = SignedEvent {
            id,
            value: event.value,
            unit: unit_str,
            stable: event.stable,
            timestamp_ms,
            hmac,
        };

        debug!(
            "[SEALER] #{} → {:.3} {} | estable={} | hmac={}...",
            signed.id,
            signed.value,
            signed.unit,
            signed.stable,
            &signed.hmac[..8]
        );

        // Guardar en buffer circular (descarta el más viejo si está lleno)
        {
            let mut buf = events.lock().await;
            if buf.len() >= max_events {
                buf.pop_front();
            }
            buf.push_back(signed.clone());
        }

        // Emitir a clientes WebSocket suscritos
        if live_tx.receiver_count() > 0 {
            if let Err(e) = live_tx.send(signed) {
                warn!("[SEALER] Error enviando a WebSocket: {}", e);
            }
        }
    }

    info!("[SEALER] Canal cerrado — deteniendo");
}

/// Genera HMAC-SHA256 sobre los campos críticos del evento.
/// Mensaje: "{id}|{value:.6}|{timestamp_ms}|{unit}"
/// Cualquier modificación posterior al id, valor, timestamp o unidad
/// invalidará este sello.
/// Verifica que un SignedEvent no fue alterado desde que fue firmado.
pub fn verify(event: &SignedEvent, key: &[u8]) -> bool {
    let expected = sign(event.id, event.value, event.timestamp_ms, &event.unit, key);
    expected == event.hmac
}

fn sign(id: u64, value: f64, timestamp_ms: u64, unit: &str, key: &[u8]) -> String {
    let message = format!("{id}|{value:.6}|{timestamp_ms}|{unit}");
    let mut mac =
        HmacSha256::new_from_slice(key).expect("HMAC acepta claves de cualquier longitud");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_produce_same_hmac() {
        let key = b"test-key";
        let h1 = sign(1, 24.55, 1700000000000, "kg", key);
        let h2 = sign(1, 24.55, 1700000000000, "kg", key);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_value_produces_different_hmac() {
        let key = b"test-key";
        let h1 = sign(1, 24.55, 1700000000000, "kg", key);
        let h2 = sign(1, 24.56, 1700000000000, "kg", key);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hmac_is_64_hex_chars() {
        let h = sign(1, 10.0, 0, "kg", b"key");
        assert_eq!(h.len(), 64);
    }
}
