use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub static CONFIG_CACHE: Mutex<Option<Arc<AppConfig>>> = Mutex::new(None);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub clipboard_shortcut: ClipboardShortcut,
    #[serde(default)]
    pub ocr_shortcut: ClipboardShortcut,
    #[serde(default)]
    pub playback_speed: f32,
    #[serde(default)]
    pub volume: f32,
    #[serde(default)]
    pub dev_mode: bool,
    #[serde(default)]
    pub source_language: String,
    #[serde(default)]
    pub target_language: String,
    #[serde(default)]
    pub tts_model_selection: String,
    #[serde(default = "default_true")]
    pub ipa_enabled: bool,
}

/// Valeur par défaut d'un booléen activé (utilisée par serde pour les
/// configurations existantes qui ne contiennent pas encore le champ).
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
            volume: 1.0,
            dev_mode: false,
            source_language: "auto".to_string(),
            target_language: "fr".to_string(),
            tts_model_selection: "piper:fr_FR-tom-medium".to_string(),
            ipa_enabled: true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_shortcut_default() {
        // Test: vérifier les valeurs par défaut du raccourci clipboard
        let default = ClipboardShortcut::default();
        assert!(default.ctrl);
        assert!(default.shift);
        assert!(!default.alt);
        assert!(!default.meta);
        assert_eq!(default.key, "V");
    }

    #[test]
    fn test_app_config_default() {
        // Test: vérifier les valeurs par défaut de la configuration
        let config = AppConfig::default();
        assert_eq!(config.playback_speed, 1.0);
        assert!(!config.dev_mode);
        assert_eq!(config.source_language, "auto");
        assert_eq!(config.target_language, "fr");
    }

    #[test]
    fn test_app_config_clipboard_shortcut_default() {
        // Test: vérifier le raccourci clipboard dans la config par défaut
        let config = AppConfig::default();
        assert!(config.clipboard_shortcut.ctrl);
        assert!(config.clipboard_shortcut.shift);
        assert_eq!(config.clipboard_shortcut.key, "V");
    }

    #[test]
    fn test_app_config_ocr_shortcut_default() {
        // Test: vérifier le raccourci OCR dans la config par défaut
        let config = AppConfig::default();
        assert!(config.ocr_shortcut.ctrl);
        assert!(!config.ocr_shortcut.shift);
        assert!(config.ocr_shortcut.alt);
        assert_eq!(config.ocr_shortcut.key, "o");
    }

    #[test]
    fn test_clipboard_shortcut_serializable() {
        // Test: vérifier que le raccourci peut être sérialisé en JSON
        let shortcut = ClipboardShortcut::default();
        let json = serde_json::to_string(&shortcut);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        let deserialized: Result<ClipboardShortcut, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap(), shortcut);
    }

    #[test]
    fn test_app_config_serializable() {
        // Test: vérifier que la configuration peut être sérialisée en JSON
        let config = AppConfig::default();
        let json = serde_json::to_string(&config);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        let deserialized: Result<AppConfig, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        let deserialized_config = deserialized.unwrap();
        assert_eq!(deserialized_config.playback_speed, config.playback_speed);
        assert_eq!(deserialized_config.dev_mode, config.dev_mode);
        assert_eq!(deserialized_config.source_language, config.source_language);
        assert_eq!(deserialized_config.target_language, config.target_language);
    }

    #[test]
    fn test_playback_speed_valid_range() {
        // Test: vérifier que la vitesse de lecture est dans une plage raisonnable
        let config = AppConfig::default();
        assert!(config.playback_speed >= 0.5);
        assert!(config.playback_speed <= 2.0);
    }

    #[test]
    fn test_app_config_source_language_auto() {
        // Test: la langue source par défaut doit être "auto"
        let config = AppConfig::default();
        assert_eq!(config.source_language, "auto");
    }

    #[test]
    fn test_app_config_target_language_french() {
        // Test: la langue cible par défaut doit être "fr"
        let config = AppConfig::default();
        assert_eq!(config.target_language, "fr");
    }
}
