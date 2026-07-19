//! Gestion persistante des raccourcis clavier

use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortcutConfig {
    pub bindings: HashMap<String, ShortcutBinding>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct ShortcutBinding {
    pub id: String,
    pub current_binding: String,
    pub default_binding: String,
}

impl ShortcutConfig {
    pub fn default() -> Self {
        let mut bindings = HashMap::new();

        bindings.insert(
            "clipboard".to_string(),
            ShortcutBinding {
                id: "clipboard".to_string(),
                current_binding: "ctrl+shift+v".to_string(),
                default_binding: "ctrl+shift+v".to_string(),
            },
        );

        bindings.insert(
            "ocr".to_string(),
            ShortcutBinding {
                id: "ocr".to_string(),
                current_binding: "ctrl+alt+o".to_string(),
                default_binding: "ctrl+alt+o".to_string(),
            },
        );

        ShortcutConfig { bindings }
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

    Ok(app_config_dir.join("shortcuts.json"))
}

pub fn load_shortcut_config() -> Result<ShortcutConfig, String> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Erreur de lecture du fichier de config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Erreur de parsing du fichier de config: {}", e))
    } else {
        Ok(ShortcutConfig::default())
    }
}

pub fn save_shortcut_config(config: &ShortcutConfig) -> Result<(), String> {
    let config_path = get_config_path()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Erreur de sérialisation: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Erreur d'écriture du fichier de config: {}", e))
}
