use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub static CONFIG_CACHE: Mutex<Option<Arc<AppConfig>>> = Mutex::new(None);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub clipboard_shortcut: ClipboardShortcut,
    #[serde(default)]
    pub ocr_shortcut: ClipboardShortcut,
    #[serde(default)]
    pub playback_speed: f32,
    #[serde(default)]
    pub dev_mode: bool,
    #[serde(default)]
    pub source_language: String,
    #[serde(default)]
    pub target_language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipboardShortcut {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
    #[serde(default)]
    pub key: String,
}

impl Default for ClipboardShortcut {
    fn default() -> Self {
        ClipboardShortcut {
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
            key: "V".to_string(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            clipboard_shortcut: ClipboardShortcut {
                ctrl: true,
                shift: true,
                alt: false,
                meta: false,
                key: "V".to_string(),
            },
            ocr_shortcut: ClipboardShortcut {
                ctrl: true,
                shift: false,
                alt: true,
                meta: false,
                key: "o".to_string(),
            },
            playback_speed: 1.0,
            dev_mode: false,
            source_language: "auto".to_string(),
            target_language: "fr".to_string(),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Impossible de trouver le répertoire de configuration".to_string())?;
    let app_config_dir = config_dir.join("gsp-ui");

    if !app_config_dir.exists() {
        std::fs::create_dir_all(&app_config_dir)
            .map_err(|e| format!("Impossible de créer le répertoire de config: {}", e))?;
    }

    Ok(app_config_dir.join("config.json"))
}

pub fn load_config() -> Result<AppConfig, String> {
    let mut cache = CONFIG_CACHE
        .lock()
        .map_err(|e| format!("Erreur de verrouillage du cache: {}", e))?;

    if let Some(cached) = cache.as_ref() {
        return Ok((**cached).clone());
    }

    let config_path = get_config_path()?;

    let config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Erreur de lecture du fichier de config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Erreur de parsing du fichier de config: {}", e))?
    } else {
        AppConfig::default()
    };

    *cache = Some(Arc::new(config.clone()));
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Erreur de sérialisation: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Erreur d'écriture du fichier de config: {}", e))?;

    let mut cache = CONFIG_CACHE
        .lock()
        .map_err(|e| format!("Erreur de verrouillage du cache: {}", e))?;
    *cache = Some(Arc::new(config.clone()));

    Ok(())
}

pub fn update_config<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut AppConfig),
{
    let mut config = load_config()?;
    f(&mut config);
    save_config(&config)
}
