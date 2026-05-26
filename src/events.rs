use serde::Serialize;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum WeightUnit {
    Kg,
    Lb,
    Ton,
    G,
    Unknown,
}

impl std::fmt::Display for WeightUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeightUnit::Kg => write!(f, "kg"),
            WeightUnit::Lb => write!(f, "lb"),
            WeightUnit::Ton => write!(f, "t"),
            WeightUnit::G => write!(f, "g"),
            WeightUnit::Unknown => write!(f, "?"),
        }
    }
}

/// Dato de peso crudo producido por el parser.
#[derive(Debug, Clone)]
pub struct WeighEvent {
    pub value: f64,
    pub unit: WeightUnit,
    pub stable: bool,
    pub timestamp: SystemTime,
    pub raw: Vec<u8>,
}

/// Dato de peso sellado con HMAC-SHA256, listo para persistir y exponer vía API.
/// El campo `hmac` garantiza que value, unit, timestamp y id no fueron alterados.
#[derive(Debug, Clone, Serialize)]
pub struct SignedEvent {
    pub id: u64,
    pub value: f64,
    pub unit: String,
    pub stable: bool,
    pub timestamp_ms: u64,
    pub hmac: String,
}
