pub mod detector;
pub mod protocols;

use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn};

use crate::events::WeighEvent;
use detector::ProtocolDetector;
use protocols::Protocol;

/// Task async del motor de parseo.
/// Consume bytes del canal broadcast de la lectura serial,
/// auto-detecta el protocolo y emite WeighEvent normalizados.
pub async fn run(mut rx: broadcast::Receiver<Vec<u8>>, event_tx: mpsc::Sender<WeighEvent>) {
    let mut buffer: Vec<u8> = Vec::with_capacity(4096);
    let mut detector = ProtocolDetector::new();
    let mut protocol: Option<Box<dyn Protocol>> = None;

    info!("[PARSER] Iniciado — esperando datos para auto-detectar protocolo...");

    loop {
        match rx.recv().await {
            Ok(bytes) => {
                buffer.extend_from_slice(&bytes);

                // Fase de aprendizaje: detectar el protocolo
                if protocol.is_none() {
                    if let Some(detected) = detector.feed(&bytes) {
                        info!("[PARSER] Protocolo confirmado: {}", detected.name());
                        protocol = Some(detected);
                        buffer.clear(); // empezar limpio con protocolo conocido
                    }
                    continue;
                }

                // Fase estable: parsear y emitir eventos
                if let Some(proto) = &protocol {
                    let (events, consumed) = proto.try_extract(&buffer);
                    buffer.drain(..consumed);

                    // Evitar que el buffer crezca sin límite ante errores
                    if buffer.len() > 16_384 {
                        warn!("[PARSER] Buffer acumulado demasiado grande — descartando datos viejos");
                        buffer.drain(..buffer.len() / 2);
                    }

                    for event in events {
                        if event_tx.send(event).await.is_err() {
                            return; // receptor cerrado
                        }
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("[PARSER] Se perdieron {} paquetes — canal saturado", n);
            }
            Err(broadcast::error::RecvError::Closed) => {
                info!("[PARSER] Canal cerrado — deteniendo parser");
                break;
            }
        }
    }
}
