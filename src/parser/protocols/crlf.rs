use crate::events::WeighEvent;
use super::{parse_ascii_frame, Protocol};
use tracing::warn;

/// Protocolo de líneas terminadas en CR/LF o LF solo.
/// Cada línea completa es una lectura de peso.
/// Usado por: indicadores modernos en modo "continuo ASCII", muchos PLC-bridge.
pub struct CrLfProtocol;

impl Protocol for CrLfProtocol {
    fn name(&self) -> &'static str {
        "CR/LF"
    }

    fn try_extract(&self, buffer: &[u8]) -> (Vec<WeighEvent>, usize) {
        let mut events = Vec::new();
        let mut consumed = 0;
        let mut pos = 0;

        while pos < buffer.len() {
            // Buscar terminador de línea (\n)
            let Some(nl_offset) = buffer[pos..].iter().position(|&b| b == b'\n') else {
                // Línea incompleta — esperar más datos
                break;
            };

            let nl_pos = pos + nl_offset;

            // El payload de la línea sin \r ni \n
            let line_end = if nl_pos > 0 && buffer[nl_pos - 1] == b'\r' {
                nl_pos - 1
            } else {
                nl_pos
            };

            let line = &buffer[pos..line_end];

            if !line.is_empty() {
                if let Some(event) = parse_ascii_frame(line) {
                    events.push(event);
                } else {
                    warn!(
                        "[CR/LF] No se pudo extraer peso de línea: {:?}",
                        String::from_utf8_lossy(line)
                    );
                }
            }

            consumed = nl_pos + 1;
            pos = consumed;
        }

        (events, consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crlf_line() {
        let data = b"S   24.550 kg\r\n";
        let proto = CrLfProtocol;
        let (events, consumed) = proto.try_extract(data);

        assert_eq!(events.len(), 1);
        assert!((events[0].value - 24.55).abs() < 0.001);
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn parses_lf_only_line() {
        let data = b"  3.200 kg\n";
        let proto = CrLfProtocol;
        let (events, consumed) = proto.try_extract(data);

        assert_eq!(events.len(), 1);
        assert!((events[0].value - 3.2).abs() < 0.001);
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn waits_for_incomplete_line() {
        let data = b"  24.550 kg"; // sin \n
        let proto = CrLfProtocol;
        let (events, consumed) = proto.try_extract(data);

        assert!(events.is_empty());
        assert_eq!(consumed, 0);
    }

    #[test]
    fn parses_multiple_lines() {
        let data = b"10.000 kg\r\n20.000 kg\r\n30.000 kg\r\n";
        let proto = CrLfProtocol;
        let (events, consumed) = proto.try_extract(data);

        assert_eq!(events.len(), 3);
        assert_eq!(consumed, data.len());
    }
}
