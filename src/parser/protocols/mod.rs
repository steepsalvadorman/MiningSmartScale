pub mod crlf;
pub mod stx_etx;

use crate::events::{WeighEvent, WeightUnit};
use std::time::SystemTime;

pub trait Protocol: Send + Sync {
    fn name(&self) -> &'static str;
    /// Intenta extraer eventos completos del buffer acumulado.
    /// Retorna (eventos_encontrados, bytes_consumidos).
    fn try_extract(&self, buffer: &[u8]) -> (Vec<WeighEvent>, usize);
}

// ── Extracción de peso desde texto ASCII ──────────────────────────────────────

pub fn parse_ascii_frame(raw: &[u8]) -> Option<WeighEvent> {
    let text: String = raw
        .iter()
        .filter(|&&b| b.is_ascii_graphic() || b == b' ')
        .map(|&b| b as char)
        .collect();

    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let value = extract_numeric(text)?;
    let unit = detect_unit(text);
    let stable = detect_stable(text);

    Some(WeighEvent {
        value,
        unit,
        stable,
        timestamp: SystemTime::now(),
        raw: raw.to_vec(),
    })
}

/// Extrae el primer número decimal válido del texto.
/// Ignora bytes de checksum (valores > 9999 o negativos imposibles).
fn extract_numeric(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        let is_sign = (b == b'-' || b == b'+')
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit();

        if b.is_ascii_digit() || is_sign {
            let start = i;
            if is_sign {
                i += 1;
            }
            let mut has_dot = false;
            let mut digit_count = 0;

            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_digit() {
                    digit_count += 1;
                    i += 1;
                } else if c == b'.' && !has_dot {
                    has_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }

            if digit_count > 0 {
                if let Ok(v) = text[start..i].parse::<f64>() {
                    // Rango razonable para cualquier balanza industrial
                    if v.abs() < 1_000_000.0 {
                        return Some(v);
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    None
}

fn detect_unit(text: &str) -> WeightUnit {
    let t = text.to_ascii_lowercase();
    if t.contains("kg") {
        WeightUnit::Kg
    } else if t.contains(" lb") || t.ends_with("lb") {
        WeightUnit::Lb
    } else if t.contains(" t ") || t.ends_with(" t") || t.contains("ton") {
        WeightUnit::Ton
    } else if t.contains(" g ") || t.ends_with(" g") {
        WeightUnit::G
    } else {
        WeightUnit::Unknown
    }
}

fn detect_stable(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    // Indicadores de peso en movimiento / inestable
    let unstable = ["us ", " d ", "mov", "uns", "dynamic", "motion", "err"];
    !unstable.iter().any(|&s| t.contains(s))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_kg_line() {
        let event = parse_ascii_frame(b"S   24.550 kg").unwrap();
        assert!((event.value - 24.55).abs() < 0.001);
        assert_eq!(event.unit, WeightUnit::Kg);
        assert!(event.stable);
    }

    #[test]
    fn parses_unstable_reading() {
        let event = parse_ascii_frame(b"US  24.550 kg").unwrap();
        assert!(!event.stable);
    }

    #[test]
    fn parses_negative_zero_tare() {
        let event = parse_ascii_frame(b"S  -0.020 kg").unwrap();
        assert!(event.value < 0.0);
    }

    #[test]
    fn returns_none_for_empty() {
        assert!(parse_ascii_frame(b"").is_none());
        assert!(parse_ascii_frame(b"   ").is_none());
    }
}
