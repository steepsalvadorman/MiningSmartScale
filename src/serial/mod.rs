use anyhow::Result;
use serialport::SerialPortType;
use tokio::io::AsyncReadExt;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use tokio_serial::SerialPortBuilderExt;
use tracing::{error, info, warn};

use crate::error::WeighFlowError;

const DEFAULT_BAUD: u32 = 9600;
const TIMEOUT_SECS: u64 = 5;
const BUF_SIZE: usize = 512;

/// Escanea los puertos disponibles y devuelve el nombre del puerto a usar.
/// Si solo hay uno, lo elige automáticamente.
/// Si hay varios, usa el primero e informa al usuario.
/// Si no hay ninguno, retorna error con mensaje descriptivo.
pub fn auto_detect() -> Result<String> {
    let ports = serialport::available_ports().unwrap_or_default();

    let candidates: Vec<String> = ports
        .into_iter()
        .filter(|p| !matches!(p.port_type, SerialPortType::BluetoothPort))
        .map(|p| p.port_name)
        .collect();

    match candidates.len() {
        0 => {
            error!("No se encontraron puertos seriales disponibles");
            error!("  → Verifica que el cable RS-232 esté conectado");
            error!("  → En Linux: el usuario debe estar en el grupo 'dialout'");
            Err(WeighFlowError::NoPorts.into())
        }
        1 => {
            info!("Puerto detectado automáticamente: {}", candidates[0]);
            Ok(candidates[0].clone())
        }
        _ => {
            warn!("Múltiples puertos encontrados: {:?}", candidates);
            info!(
                "Usando '{}' — ejecuta con el nombre del puerto como argumento para elegir otro",
                candidates[0]
            );
            Ok(candidates[0].clone())
        }
    }
}

/// Task async que lee bytes del puerto serial de forma continua
/// y los publica en el canal broadcast para que el parser y otros
/// consumidores los reciban.
pub async fn read_loop(port_name: &str, tx: broadcast::Sender<Vec<u8>>) -> Result<()> {
    info!("Abriendo '{}' a {} baud...", port_name, DEFAULT_BAUD);

    let mut port = tokio_serial::new(port_name, DEFAULT_BAUD)
        .open_native_async()
        .map_err(|e| WeighFlowError::PortOpen {
            port: port_name.to_string(),
            reason: e.to_string(),
        })?;

    info!("Conectado — leyendo datos de balanza");
    info!("─────────────────────────────────────────────────");

    let mut buf = vec![0u8; BUF_SIZE];

    loop {
        match timeout(Duration::from_secs(TIMEOUT_SECS), port.read(&mut buf)).await {
            Ok(Ok(0)) => {
                warn!("Puerto cerrado por el dispositivo");
                return Err(WeighFlowError::Disconnected.into());
            }
            Ok(Ok(n)) => {
                let data = buf[..n].to_vec();
                if tx.send(data).is_err() {
                    // No hay receptores activos, continuar igual
                }
            }
            Ok(Err(e)) => {
                error!("Error de lectura en '{}': {}", port_name, e);
                return Err(WeighFlowError::Io(e).into());
            }
            Err(_) => {
                warn!(
                    "[TIMEOUT] Sin datos en {}s — balanza apagada o sin señal",
                    TIMEOUT_SECS
                );
            }
        }
    }
}
