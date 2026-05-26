use thiserror::Error;

#[derive(Debug, Error)]
pub enum WeighFlowError {
    #[error("No se encontraron puertos seriales — verifica que el cable RS-232 esté conectado")]
    NoPorts,

    #[error("No se pudo abrir el puerto '{port}': {reason}")]
    PortOpen { port: String, reason: String },

    #[error("Puerto serial desconectado — verifica el cable RS-232")]
    Disconnected,

    #[error("Sin datos en {seconds}s — balanza apagada o sin señal")]
    Timeout { seconds: u64 },

    #[error("Error de I/O en puerto serial: {0}")]
    Io(#[from] std::io::Error),
}
