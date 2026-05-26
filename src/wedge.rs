use enigo::{Direction::Click, Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::config::WedgeConfig;
use crate::events::SignedEvent;

/// Task async del wedge mode.
///
/// Escucha eventos de peso y cuando llega uno estable:
///   1. Simula el teclado para escribir el valor en la aplicación activa
///      (Excel, LibreOffice, SAP, cualquier campo con foco).
///   2. Opcionalmente copia el valor al portapapeles.
///
/// Funciona sin modificar la aplicación destino — como un lector de código de barras.
pub async fn run(mut rx: broadcast::Receiver<SignedEvent>, cfg: WedgeConfig) {
    if !cfg.enabled {
        info!("[WEDGE] Modo teclado deshabilitado (enabled = false en weighflow.toml)");
        return;
    }

    // Inicializar simulador de teclado
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => {
            info!("[WEDGE] Simulador de teclado iniciado — separador: '{}'", cfg.separator);
            if cfg.clipboard {
                info!("[WEDGE] Portapapeles habilitado");
            }
            e
        }
        Err(e) => {
            error!("[WEDGE] No se pudo inicializar el simulador de teclado: {}", e);
            error!("[WEDGE]   → En Linux/Wayland: verifica acceso a /dev/uinput");
            error!("[WEDGE]   → En Linux/X11: verifica que DISPLAY esté configurado");
            error!("[WEDGE]   → Wedge mode deshabilitado para esta sesión");
            return;
        }
    };

    let min_interval = Duration::from_millis(cfg.min_interval_ms);
    let mut last_typed_at: Option<Instant> = None;

    while let Ok(event) = rx.recv().await {
        // Filtrar lecturas inestables si corresponde
        if cfg.stable_only && !event.stable {
            debug!("[WEDGE] Lectura inestable ignorada: {:.3}", event.value);
            continue;
        }

        // Respetar intervalo mínimo para no teclear el mismo peso 20 veces por segundo
        if let Some(last) = last_typed_at {
            if last.elapsed() < min_interval {
                continue;
            }
        }

        let weight_str = format!("{:.3}", event.value);

        // Portapapeles (no falla si no está disponible)
        if cfg.clipboard {
            copy_to_clipboard(&weight_str, &event.unit);
        }

        // Inyección de teclado
        match enigo.text(&weight_str) {
            Ok(_) => {
                type_separator(&mut enigo, &cfg.separator);
                last_typed_at = Some(Instant::now());
                info!(
                    "[WEDGE] ✓ Tecleado: {} {} → separador '{}'",
                    weight_str, event.unit, cfg.separator
                );
            }
            Err(e) => {
                warn!("[WEDGE] Error al inyectar teclado: {}", e);
            }
        }
    }
}

fn type_separator(enigo: &mut Enigo, separator: &str) {
    match separator {
        "tab" => {
            let _ = enigo.key(Key::Tab, Click);
        }
        "enter" => {
            let _ = enigo.key(Key::Return, Click);
        }
        "tab_enter" | "tab+enter" => {
            let _ = enigo.key(Key::Tab, Click);
            let _ = enigo.key(Key::Return, Click);
        }
        other => {
            warn!("[WEDGE] Separador desconocido '{}' — sin separador", other);
        }
    }
}

fn copy_to_clipboard(weight_str: &str, unit: &str) {
    match arboard::Clipboard::new() {
        Ok(mut cb) => {
            let text = format!("{} {}", weight_str, unit);
            if let Err(e) = cb.set_text(&text) {
                warn!("[CLIPBOARD] Error al copiar: {}", e);
            } else {
                debug!("[CLIPBOARD] Copiado: {}", text);
            }
        }
        Err(e) => {
            warn!("[CLIPBOARD] No disponible: {} — verifica X11/Wayland", e);
        }
    }
}
