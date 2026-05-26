use serde::Deserialize;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub serial: SerialConfig,
    pub security: SecurityConfig,
    pub server: ServerConfig,
    pub wedge: WedgeConfig,
    pub export: ExportConfig,
}

#[derive(Debug, Deserialize)]
pub struct SerialConfig {
    /// Puerto serial a usar. None = auto-detectar.
    pub port: Option<String>,
    pub baud_rate: u32,
}

#[derive(Debug, Deserialize)]
pub struct SecurityConfig {
    /// Clave secreta para HMAC-SHA256. Cambiar en producción.
    pub hmac_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub http_port: u16,
    pub event_buffer_size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WedgeConfig {
    /// Habilita el modo teclado virtual (wedge mode).
    pub enabled: bool,
    /// Separador que se escribe después del peso: "tab", "enter", "tab_enter".
    pub separator: String,
    /// Si true, solo escribe cuando la lectura es estable.
    pub stable_only: bool,
    /// Si true, copia el peso al portapapeles además de teclear.
    pub clipboard: bool,
    /// Milisegundos mínimos entre inyecciones de teclado consecutivas.
    pub min_interval_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExportConfig {
    /// Directorio donde se guarda el CSV al exportar.
    pub output_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            serial: SerialConfig {
                port: None,
                baud_rate: 9600,
            },
            security: SecurityConfig {
                hmac_key: "weighflow-change-in-production".to_string(),
            },
            server: ServerConfig {
                http_port: 8080,
                event_buffer_size: 1000,
            },
            wedge: WedgeConfig {
                enabled: false,
                separator: "tab".to_string(),
                stable_only: true,
                clipboard: false,
                min_interval_ms: 500,
            },
            export: ExportConfig {
                output_dir: ".".to_string(),
            },
        }
    }
}

impl Config {
    /// Carga la configuración desde una ruta explícita o busca en rutas estándar:
    ///   1. ruta explícita (--config)
    ///   2. ./weighflow.toml           (desarrollo)
    ///   3. /etc/weighflow/weighflow.toml  (servicio del sistema)
    ///   4. ~/.config/weighflow/weighflow.toml  (instalación de usuario)
    pub fn load(explicit: Option<&str>) -> Self {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        if let Some(p) = explicit {
            candidates.push(std::path::PathBuf::from(p));
        } else {
            // 1. Directorio actual (desarrollo / arranque manual)
            candidates.push(std::path::PathBuf::from("weighflow.toml"));

            // 2. Rutas del sistema según plataforma
            #[cfg(target_os = "windows")]
            {
                // C:\ProgramData\WeighFlow\weighflow.toml  (instalación de sistema)
                if let Ok(pd) = std::env::var("PROGRAMDATA") {
                    candidates.push(
                        std::path::PathBuf::from(pd).join("WeighFlow\\weighflow.toml"),
                    );
                }
                // %APPDATA%\WeighFlow\weighflow.toml  (instalación de usuario)
                if let Ok(ad) = std::env::var("APPDATA") {
                    candidates.push(
                        std::path::PathBuf::from(ad).join("WeighFlow\\weighflow.toml"),
                    );
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                // /etc/weighflow/weighflow.toml  (servicio systemd)
                candidates.push(std::path::PathBuf::from("/etc/weighflow/weighflow.toml"));
                // ~/.config/weighflow/weighflow.toml  (instalación de usuario)
                if let Ok(home) = std::env::var("HOME") {
                    candidates.push(
                        std::path::PathBuf::from(home)
                            .join(".config/weighflow/weighflow.toml"),
                    );
                }
            }
        }

        for path in &candidates {
            if !path.exists() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(cfg) => {
                        info!("Configuración cargada desde {}", path.display());
                        return cfg;
                    }
                    Err(e) => {
                        warn!("[CONFIG] Error en {}: {} — usando defaults", path.display(), e);
                        return Config::default();
                    }
                },
                Err(e) => {
                    warn!("[CONFIG] No se pudo leer {}: {}", path.display(), e);
                }
            }
        }

        info!("weighflow.toml no encontrado — usando configuración por defecto");
        Config::default()
    }
}
