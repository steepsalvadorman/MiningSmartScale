use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};
use tracing::info;

use crate::events::SignedEvent;

// ── CSV ───────────────────────────────────────────────────────────────────────

/// Serializa los eventos del turno a formato CSV.
/// Compatible con Excel, LibreOffice Calc, y cualquier hoja de cálculo.
pub fn to_csv(events: &VecDeque<SignedEvent>) -> String {
    let mut out = String::with_capacity(events.len() * 80);

    writeln!(
        out,
        "id,fecha_hora,timestamp_ms,valor,unidad,estable,hmac"
    )
    .unwrap();

    for e in events {
        let dt = format_timestamp(e.timestamp_ms);
        writeln!(
            out,
            "{},{},{},{:.6},{},{},{}",
            e.id, dt, e.timestamp_ms, e.value, e.unit, e.stable, e.hmac
        )
        .unwrap();
    }

    out
}

/// Guarda el CSV en disco en el directorio especificado.
/// El nombre del archivo incluye la fecha para facilitar el archivo de turnos.
pub fn save_csv(events: &VecDeque<SignedEvent>, output_dir: &str) -> anyhow::Result<String> {
    let dir = Path::new(output_dir);
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }

    let filename = format!("pesajes_{}.csv", today_str());
    let path = dir.join(&filename);

    let csv = to_csv(events);
    let mut file = std::fs::File::create(&path)?;
    file.write_all(csv.as_bytes())?;

    let full_path = path.to_string_lossy().to_string();
    info!("[EXPORT] CSV guardado: {} ({} registros)", full_path, events.len());
    Ok(full_path)
}

// ── Utilidades ────────────────────────────────────────────────────────────────

fn format_timestamp(ms: u64) -> String {
    let secs = ms / 1000;
    let dt = UNIX_EPOCH + Duration::from_secs(secs);
    // Formatear manualmente sin chrono para mantener las dependencias mínimas
    // Día relativo desde epoch — suficiente para identificar turnos
    let total_secs = dt
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (total_secs % 86400) / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    let days = total_secs / 86400;
    // Aproximación: días desde 1970-01-01 → fecha gregoriana
    let (year, month, day) = days_to_date(days);
    format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{s:02}")
}

pub fn today_str() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let (y, m, d) = days_to_date(days);
    format!("{y:04}{m:02}{d:02}")
}

/// Convierte días desde Unix epoch a (año, mes, día).
fn days_to_date(days: u64) -> (u64, u64, u64) {
    let mut remaining = days;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let month_days: &[u64] = if is_leap(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    (year, month, remaining + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(id: u64, value: f64, unit: &str, stable: bool) -> SignedEvent {
        SignedEvent {
            id,
            value,
            unit: unit.to_string(),
            stable,
            timestamp_ms: 1700000000000 + id * 60_000,
            hmac: "a".repeat(64),
        }
    }

    #[test]
    fn csv_tiene_encabezado_correcto() {
        let events = VecDeque::new();
        let csv = to_csv(&events);
        assert!(csv.starts_with("id,fecha_hora,timestamp_ms,valor,unidad,estable,hmac\n"));
    }

    #[test]
    fn csv_genera_una_fila_por_evento() {
        let mut events = VecDeque::new();
        events.push_back(make_event(1, 24.55, "kg", true));
        events.push_back(make_event(2, 18.20, "kg", false));

        let csv = to_csv(&events);
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(lines.len(), 3); // encabezado + 2 filas
        assert!(lines[1].starts_with("1,"));
        assert!(lines[2].starts_with("2,"));
    }

    #[test]
    fn csv_incluye_valor_con_6_decimales() {
        let mut events = VecDeque::new();
        events.push_back(make_event(1, 24.55, "kg", true));
        let csv = to_csv(&events);
        assert!(csv.contains("24.550000"), "CSV debe tener 6 decimales: {}", csv);
    }

    #[test]
    fn csv_incluye_estado_estable() {
        let mut events = VecDeque::new();
        events.push_back(make_event(1, 10.0, "kg", true));
        events.push_back(make_event(2, 10.0, "kg", false));
        let csv = to_csv(&events);
        assert!(csv.contains(",true,"));
        assert!(csv.contains(",false,"));
    }

    #[test]
    fn format_timestamp_produce_fecha_legible() {
        // 1700000000000 ms = 2023-11-14 22:13:20 UTC
        let s = format_timestamp(1700000000000);
        assert!(s.starts_with("2023-11-14"), "Fecha inesperada: {}", s);
    }

    #[test]
    fn csv_buffer_vacio_solo_tiene_encabezado() {
        let events = VecDeque::new();
        let csv = to_csv(&events);
        assert_eq!(csv.lines().count(), 1);
    }

    #[test]
    fn save_csv_escribe_archivo_en_disco() {
        let mut events = VecDeque::new();
        events.push_back(make_event(1, 33.3, "kg", true));
        events.push_back(make_event(2, 44.4, "lb", false));

        let tmp = std::env::temp_dir().join("weighflow_test_export");
        let dir = tmp.to_str().unwrap();

        let path = save_csv(&events, dir).expect("save_csv no debe fallar");
        assert!(std::path::Path::new(&path).exists(), "Archivo CSV debe existir en: {}", path);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("id,fecha_hora"), "Debe tener encabezado");
        assert!(content.contains("33.300000"), "Debe contener peso 33.3");
        assert!(content.contains("44.400000"), "Debe contener peso 44.4");
        assert!(content.contains(",kg,"), "Debe contener unidad kg");
        assert!(content.contains(",lb,"), "Debe contener unidad lb");

        // Limpieza
        let _ = std::fs::remove_file(&path);
    }
}
