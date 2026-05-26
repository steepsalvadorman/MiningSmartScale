use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use weighflow::{api, config, events, parser, sealer, serial, wedge};
use events::{SignedEvent, WeighEvent};

/// Parsea argumentos simples de línea de comandos.
///
/// Soportado:
///   weighflow [--config PATH] [--port DEVICE] [DEVICE]
///
/// El DEVICE posicional se mantiene por compatibilidad con la etapa anterior.
fn parse_args() -> (Option<String>, Option<String>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path: Option<String> = None;
    let mut port: Option<String> = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(args[i].clone());
                }
            }
            "--port" => {
                i += 1;
                if i < args.len() {
                    port = Some(args[i].clone());
                }
            }
            arg if !arg.starts_with('-') && port.is_none() => {
                port = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    (config_path, port)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .compact()
        .init();

    info!("WeighFlow IoT v0.5 — iniciando");
    info!("─────────────────────────────────────────────────");

    let (config_path, cli_port) = parse_args();
    let cfg = config::Config::load(config_path.as_deref());

    let port_name = match cli_port.or_else(|| cfg.serial.port.clone()) {
        Some(p) => { info!("Puerto: {}", p); p }
        None => serial::auto_detect()?,
    };

    let state = Arc::new(api::AppState::new(cfg.server.event_buffer_size));

    let (raw_tx, raw_rx) = broadcast::channel::<Vec<u8>>(256);
    let (event_tx, event_rx) = mpsc::channel::<WeighEvent>(64);

    tokio::spawn(parser::run(raw_rx, event_tx));

    let hmac_key = cfg.security.hmac_key.into_bytes();
    tokio::spawn(sealer::run(
        event_rx,
        state.events.clone(),
        state.live_tx.clone(),
        state.max_events,
        hmac_key,
    ));

    tokio::spawn(wedge::run(state.live_tx.subscribe(), cfg.wedge));
    tokio::spawn(print_events(state.live_tx.subscribe()));
    tokio::spawn(api::serve(state, cfg.server.http_port));

    if let Err(e) = serial::read_loop(&port_name, raw_tx).await {
        error!("[CRÍTICO] {}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn print_events(mut rx: broadcast::Receiver<SignedEvent>) {
    while let Ok(event) = rx.recv().await {
        let estado = if event.stable { "ESTABLE" } else { "EN MOVIMIENTO" };
        if event.stable {
            info!("PESO #{:<4} │ {:>10.3} {} │ {}", event.id, event.value, event.unit, estado);
        } else {
            warn!("PESO #{:<4} │ {:>10.3} {} │ {}", event.id, event.value, event.unit, estado);
        }
    }
}
