use tracing::{debug, info, warn};

use super::protocols::{crlf::CrLfProtocol, stx_etx::StxEtxProtocol, Protocol};

/// Número mínimo de delimitadores observados para decidir un protocolo.
const MIN_BOUNDARIES: usize = 3;

/// Límite del buffer de aprendizaje para evitar crecimiento ilimitado.
const MAX_LEARN_BYTES: usize = 8192;

pub struct ProtocolDetector {
    buffer: Vec<u8>,
    stx_count: usize,
    etx_count: usize,
    crlf_count: usize,
    lf_only_count: usize,
    bytes_seen: usize,
}

impl ProtocolDetector {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            stx_count: 0,
            etx_count: 0,
            crlf_count: 0,
            lf_only_count: 0,
            bytes_seen: 0,
        }
    }

    /// Alimenta el detector con nuevos bytes.
    /// Retorna Some(protocol) cuando tiene suficiente evidencia para decidir.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<Box<dyn Protocol>> {
        for &b in bytes {
            self.bytes_seen += 1;

            match b {
                0x02 => {
                    self.stx_count += 1;
                    debug!("STX detectado (total: {})", self.stx_count);
                }
                0x03 => {
                    self.etx_count += 1;
                    debug!("ETX detectado (total: {})", self.etx_count);
                }
                b'\n' => {
                    let prev = self.buffer.last().copied().unwrap_or(0);
                    if prev == b'\r' {
                        self.crlf_count += 1;
                        debug!("CR/LF detectado (total: {})", self.crlf_count);
                    } else {
                        self.lf_only_count += 1;
                        debug!("LF detectado (total: {})", self.lf_only_count);
                    }
                }
                _ => {}
            }

            self.buffer.push(b);
        }

        // Evitar crecimiento ilimitado del buffer de aprendizaje
        if self.buffer.len() > MAX_LEARN_BYTES {
            self.buffer.drain(..MAX_LEARN_BYTES / 2);
        }

        self.decide()
    }

    fn decide(&self) -> Option<Box<dyn Protocol>> {
        // STX/ETX tiene prioridad: si vemos pares coherentes, es definitivo
        if self.stx_count >= MIN_BOUNDARIES && self.etx_count >= MIN_BOUNDARIES {
            info!(
                "Protocolo auto-detectado: STX/ETX ({} STX, {} ETX en {} bytes)",
                self.stx_count, self.etx_count, self.bytes_seen
            );
            return Some(Box::new(StxEtxProtocol));
        }

        if self.crlf_count >= MIN_BOUNDARIES {
            info!(
                "Protocolo auto-detectado: CR/LF ({} líneas en {} bytes)",
                self.crlf_count, self.bytes_seen
            );
            return Some(Box::new(CrLfProtocol));
        }

        if self.lf_only_count >= MIN_BOUNDARIES {
            info!(
                "Protocolo auto-detectado: LF ({} líneas en {} bytes)",
                self.lf_only_count, self.bytes_seen
            );
            return Some(Box::new(CrLfProtocol));
        }

        // Si llevamos muchos bytes sin detectar nada, advertir
        if self.bytes_seen > 0 && self.bytes_seen % 512 == 0 {
            warn!(
                "[DETECTOR] {} bytes analizados sin protocolo claro \
                 (STX={}, ETX={}, CRLF={}, LF={}) — verifica configuración de balanza",
                self.bytes_seen,
                self.stx_count,
                self.etx_count,
                self.crlf_count,
                self.lf_only_count,
            );
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_stx_etx_after_3_frames() {
        let mut detector = ProtocolDetector::new();

        for _ in 0..3 {
            let mut frame = vec![0x02];
            frame.extend_from_slice(b"  24.550 kg");
            frame.push(0x03);
            detector.feed(&frame);
        }

        let result = detector.feed(&[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "STX/ETX");
    }

    #[test]
    fn detects_crlf_after_3_lines() {
        let mut detector = ProtocolDetector::new();

        for _ in 0..3 {
            detector.feed(b"  24.550 kg\r\n");
        }

        let result = detector.feed(&[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "CR/LF");
    }

    #[test]
    fn returns_none_without_enough_data() {
        let mut detector = ProtocolDetector::new();
        detector.feed(b"\x02  24.550 kg\x03");
        // Solo 1 frame STX/ETX, necesita MIN_BOUNDARIES=3
        assert!(detector.feed(&[]).is_none());
    }
}
