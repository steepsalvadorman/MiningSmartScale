use crate::events::WeighEvent;
use super::{parse_ascii_frame, Protocol};
use tracing::warn;

/// Protocolo STX (0x02) ... payload ... ETX (0x03) [checksum opcional]
/// Usado por: Mettler Toledo, Rinstrum, CAS, y la mayoría de indicadores industriales.
pub struct StxEtxProtocol;

impl Protocol for StxEtxProtocol {
    fn name(&self) -> &'static str {
        "STX/ETX"
    }

    fn try_extract(&self, buffer: &[u8]) -> (Vec<WeighEvent>, usize) {
        let mut events = Vec::new();
        let mut consumed = 0;
        let mut pos = 0;

        while pos < buffer.len() {
            // Buscar STX
            let Some(stx_offset) = buffer[pos..].iter().position(|&b| b == 0x02) else {
                // No hay STX — descartar todo excepto los últimos bytes por si llega uno partido
                consumed = buffer.len().saturating_sub(1);
                break;
            };

            let stx_pos = pos + stx_offset;

            // Buscar ETX después del STX
            let Some(etx_offset) = buffer[stx_pos + 1..].iter().position(|&b| b == 0x03) else {
                // Trama incompleta — esperar más datos
                consumed = stx_pos; // descartar lo que hay antes del STX
                break;
            };

            let etx_pos = stx_pos + 1 + etx_offset;
            let payload = &buffer[stx_pos + 1..etx_pos];

            if let Some(event) = parse_ascii_frame(payload) {
                events.push(event);
            } else {
                warn!(
                    "[STX/ETX] No se pudo extraer peso del payload: {:02X?}",
                    payload
                );
            }

            // Saltar STX + payload + ETX + checksum opcional (1 byte)
            let mut next = etx_pos + 1;
            if next < buffer.len() && !matches!(buffer[next], 0x02 | 0x0D | 0x0A) {
                next += 1; // byte de checksum
            }

            consumed = next;
            pos = next;
        }

        (events, consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_frame() {
        // STX + "  24.550 kg S" + ETX + checksum
        let mut frame = vec![0x02];
        frame.extend_from_slice(b"  24.550 kg S");
        frame.push(0x03);
        frame.push(0x45); // checksum ficticio

        let proto = StxEtxProtocol;
        let (events, consumed) = proto.try_extract(&frame);

        assert_eq!(events.len(), 1);
        assert!((events[0].value - 24.55).abs() < 0.001);
        assert_eq!(consumed, frame.len());
    }

    #[test]
    fn waits_for_incomplete_frame() {
        let frame = vec![0x02, b'2', b'4', b'.', b'5']; // sin ETX
        let proto = StxEtxProtocol;
        let (events, consumed) = proto.try_extract(&frame);

        assert!(events.is_empty());
        assert_eq!(consumed, 0);
    }

    #[test]
    fn extracts_multiple_frames() {
        let mut data = Vec::new();
        for _ in 0..3 {
            data.push(0x02);
            data.extend_from_slice(b"  10.000 kg S");
            data.push(0x03);
            data.push(0xAB);
        }

        let proto = StxEtxProtocol;
        let (events, consumed) = proto.try_extract(&data);

        assert_eq!(events.len(), 3);
        assert_eq!(consumed, data.len());
    }
}
