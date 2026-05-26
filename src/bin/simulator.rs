/// WeighFlow Simulator — Balanza industrial virtual para pruebas sin hardware.
///
/// Crea un par de puertos seriales virtuales (PTY) en Linux.
/// El simulador escribe tramas de peso en un extremo; el otro extremo
/// aparece como /dev/pts/N y puede pasarse directamente a weighflow.
///
/// Uso:
///   cargo run --bin simulator                  # protocolo CRLF, variación aleatoria
///   cargo run --bin simulator -- --stx-etx    # protocolo STX/ETX
///   cargo run --bin simulator -- --fixed 25.5 # peso fijo sin variación
use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn sigint_handler(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stx_etx = args.iter().any(|a| a == "--stx-etx");
    let fixed_weight: Option<f64> = args
        .windows(2)
        .find(|w| w[0] == "--fixed")
        .and_then(|w| w[1].parse().ok());

    let protocol_name = if use_stx_etx { "STX/ETX (Rinstrum)" } else { "CR/LF (Mettler Toledo)" };

    println!("╔══════════════════════════════════════════════╗");
    println!("║       WeighFlow IoT — Simulador de Balanza   ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("  Protocolo : {}", protocol_name);
    println!("  Intervalo : 500ms");
    println!();

    let (master_fd, slave_path) = create_pty().expect("No se pudo crear PTY virtual");

    println!("  Puerto virtual: \x1b[1;32m{}\x1b[0m", slave_path);
    println!();
    println!("  Ejecuta en otra terminal:");
    println!("  \x1b[1;36m./target/debug/weighflow {}\x1b[0m", slave_path);
    println!();
    println!("  Presiona Ctrl+C para detener");
    println!("──────────────────────────────────────────────────");

    unsafe { libc::signal(libc::SIGINT, sigint_handler as *const () as libc::sighandler_t) };

    let mut writer = unsafe { File::from_raw_fd(master_fd) };
    let mut base_weight: f64 = fixed_weight.unwrap_or(24.550);
    let mut tick: u64 = 0;

    while RUNNING.load(Ordering::SeqCst) {
        // Simular variación de peso (±0.050 kg) excepto si es peso fijo
        if fixed_weight.is_none() {
            let variation = ((tick as f64 * 0.37).sin() * 0.025)
                + ((tick as f64 * 1.13).cos() * 0.015);
            base_weight = 24.550 + variation;
        }

        // Peso inestable cada 7 ticks (simula camión en movimiento)
        let stable = tick % 7 != 0;
        let status = if stable { "S" } else { "US" };

        let frame: Vec<u8> = if use_stx_etx {
            build_stx_etx(base_weight, status)
        } else {
            build_crlf(base_weight, status)
        };

        let weight_display = format!("{:.3} kg", base_weight);
        let state_str = if stable { "\x1b[32mESTABLE\x1b[0m    " } else { "\x1b[33mEN MOVIMIENTO\x1b[0m" };
        println!("  [{}]  {:>12}  {}", tick_time(), weight_display, state_str);

        if let Err(e) = writer.write_all(&frame) {
            eprintln!("  [ERROR] Error escribiendo al PTY: {}", e);
            break;
        }

        tick += 1;
        std::thread::sleep(Duration::from_millis(500));
    }

    println!();
    println!("  Simulador detenido.");
}

// ── Constructores de trama ─────────────────────────────────────────────────────

fn build_crlf(weight: f64, status: &str) -> Vec<u8> {
    format!("{status}   {weight:.3} kg\r\n").into_bytes()
}

fn build_stx_etx(weight: f64, status: &str) -> Vec<u8> {
    let payload = format!("{status}   {weight:.3} kg");
    let checksum = payload.bytes().fold(0u8, |acc, b| acc ^ b);
    let mut frame = vec![0x02];
    frame.extend_from_slice(payload.as_bytes());
    frame.push(0x03);
    frame.push(checksum);
    frame
}

// ── PTY virtual ───────────────────────────────────────────────────────────────

fn create_pty() -> Result<(i32, String), String> {
    let fd = unsafe { libc::open(b"/dev/ptmx\0".as_ptr() as *const libc::c_char, libc::O_RDWR | libc::O_NOCTTY) };
    if fd < 0 {
        return Err("No se pudo abrir /dev/ptmx".into());
    }

    if unsafe { libc::grantpt(fd) } != 0 {
        return Err("grantpt falló".into());
    }
    if unsafe { libc::unlockpt(fd) } != 0 {
        return Err("unlockpt falló".into());
    }

    let mut buf = vec![0i8; 64];
    if unsafe { libc::ptsname_r(fd, buf.as_mut_ptr(), buf.len()) } != 0 {
        return Err("ptsname_r falló".into());
    }

    let slave_path = unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .to_string();

    Ok((fd, slave_path))
}

// ── Utilidades ────────────────────────────────────────────────────────────────

fn tick_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

