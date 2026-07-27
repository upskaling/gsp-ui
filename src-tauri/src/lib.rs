mod language_detector;
mod model_downloader;
mod ocr;
mod screenshooter;
mod shortcut;
pub mod textutils;
mod translator;
mod translation_engine;
mod tts;

use language_detector::{detect_language, DetectedLanguage};
use log::{debug, error, info};
use ocr::tesseract;
use rodio::{Decoder, DeviceSinkBuilder, Player};
use screenshooter::screenshot_region;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri::{AppHandle, Emitter, State};
use textutils::{preprocess_text, read_vars};
use translator::translate;
#[cfg(target_os = "linux")]
use tts::EspeakNg;
#[cfg(target_os = "macos")]
use tts::MacOsTts;
use tts::TtsEngine;

struct AudioPlayback {
    player: Player,
    _device_sink: Box<dyn std::any::Any + Send>,
    #[allow(dead_code)]
    thread_id: u128,
}

static CURRENT_SINK: Mutex<Option<AudioPlayback>> = Mutex::new(None);
static CURRENT_THREAD_ID: Mutex<u128> = Mutex::new(0);
static CONFIG_CACHE: Mutex<Option<Arc<AppConfig>>> = Mutex::new(None);

struct PlaybackState {
    _dummy: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    #[serde(default)]
    clipboard_shortcut: ClipboardShortcut,
    #[serde(default)]
    ocr_shortcut: ClipboardShortcut,
    #[serde(default)]
    playback_speed: f32,
    #[serde(default)]
    dev_mode: bool,
    #[serde(default)]
    source_language: String,
    #[serde(default)]
    target_language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ClipboardShortcut {
    #[serde(default)]
    ctrl: bool,
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    meta: bool,
    #[serde(default)]
    key: String,
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

fn get_config_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Impossible de trouver le répertoire de configuration".to_string())?;
    let app_config_dir = config_dir.join("gsp-ui");

    if !app_config_dir.exists() {
        std::fs::create_dir_all(&app_config_dir)
            .map_err(|e| format!("Impossible de créer le répertoire de config: {}", e))?;
    }

    Ok(app_config_dir.join("config.json"))
}

fn load_config() -> Result<AppConfig, String> {
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

fn save_config(config: &AppConfig) -> Result<(), String> {
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

fn update_config<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut AppConfig),
{
    let mut config = load_config()?;
    f(&mut config);
    save_config(&config)
}

fn load_icon_for_tray() -> Result<Image<'static>, tauri::Error> {
    let icon_bytes = include_bytes!("../icons/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| tauri::Error::AssetNotFound(format!("Failed to load icon: {}", e)))?;

    let img_rgba = img.to_rgba8();
    let (width, height) = img_rgba.dimensions();

    let rgba_data = img_rgba.into_raw();
    let static_data: &'static [u8] = Box::leak(rgba_data.into_boxed_slice());

    Ok(Image::new(static_data, width, height))
}

fn setup_global_shortcut(app: &tauri::App) -> Result<(), String> {
    info!("Début de setup_global_shortcut()");

    // Migrer la configuration existante si nécessaire
    if let Err(e) = migrate_old_shortcut_config() {
        error!("Erreur de migration de la configuration: {}", e);
    }

    shortcut::init_shortcuts(app.handle());
    Ok(())
}

fn migrate_old_shortcut_config() -> Result<(), String> {
    use std::collections::HashMap;

    let config = load_config()?;
    let shortcut_config_path = shortcut::settings::get_config_path()?;

    if !shortcut_config_path.exists() {
        // Créer la nouvelle configuration à partir de l'ancienne
        let mut bindings = HashMap::new();

        bindings.insert(
            "clipboard".to_string(),
            shortcut::ShortcutBinding {
                id: "clipboard".to_string(),
                current_binding: format!(
                    "{}{}{}{}",
                    if config.clipboard_shortcut.ctrl {
                        "ctrl+"
                    } else {
                        ""
                    },
                    if config.clipboard_shortcut.shift {
                        "shift+"
                    } else {
                        ""
                    },
                    if config.clipboard_shortcut.alt {
                        "alt+"
                    } else {
                        ""
                    },
                    config.clipboard_shortcut.key
                ),
                default_binding: "ctrl+shift+v".to_string(),
            },
        );

        bindings.insert(
            "ocr".to_string(),
            shortcut::ShortcutBinding {
                id: "ocr".to_string(),
                current_binding: format!(
                    "{}{}{}{}",
                    if config.ocr_shortcut.ctrl {
                        "ctrl+"
                    } else {
                        ""
                    },
                    if config.ocr_shortcut.shift {
                        "shift+"
                    } else {
                        ""
                    },
                    if config.ocr_shortcut.alt { "alt+" } else { "" },
                    config.ocr_shortcut.key
                ),
                default_binding: "ctrl+alt+o".to_string(),
            },
        );

        let shortcut_config = shortcut::ShortcutConfig { bindings };
        shortcut::settings::save_shortcut_config(&shortcut_config)?;
    }

    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();

    let show = MenuItem::with_id(app, "show", "Afficher", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let sep = tauri::menu::PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&show, &sep, &quit])?;

    let icon = load_icon_for_tray()?;

    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(icon)
        .on_menu_event(move |app, event| match event.id.0.as_str() {
            "quit" => {
                app.exit(0);
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(move |_app, event| {
            use tauri::tray::TrayIconEvent;

            if matches!(event, TrayIconEvent::Click { .. }) {
                if let Some(window) = app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn load_shortcut_config() -> Result<ClipboardShortcut, String> {
    let config = shortcut::settings::load_shortcut_config()?;
    let binding = config
        .bindings
        .get("clipboard")
        .ok_or("Raccourci clipboard non trouvé")?;

    // Parser le binding en format ClipboardShortcut
    parse_binding_to_shortcut(&binding.current_binding)
}

#[tauri::command]
fn save_shortcut_config(app_handle: AppHandle, shortcut: ClipboardShortcut) -> Result<(), String> {
    // Créer la chaîne de raccourci
    let shortcut_str = format!(
        "{}{}{}{}",
        if shortcut.ctrl { "ctrl+" } else { "" },
        if shortcut.shift { "shift+" } else { "" },
        if shortcut.alt { "alt+" } else { "" },
        shortcut.key
    );

    // Enregistrer le nouveau raccourci via le module shortcut
    shortcut::change_binding(app_handle, "clipboard".to_string(), shortcut_str)?;

    Ok(())
}

#[tauri::command]
fn load_ocr_shortcut_config() -> Result<ClipboardShortcut, String> {
    let config = shortcut::settings::load_shortcut_config()?;
    let binding = config
        .bindings
        .get("ocr")
        .ok_or("Raccourci ocr non trouvé")?;

    // Parser le binding en format ClipboardShortcut
    parse_binding_to_shortcut(&binding.current_binding)
}

#[tauri::command]
fn save_ocr_shortcut_config(
    app_handle: AppHandle,
    shortcut: ClipboardShortcut,
) -> Result<(), String> {
    // Créer la chaîne de raccourci
    let shortcut_str = format!(
        "{}{}{}{}",
        if shortcut.ctrl { "ctrl+" } else { "" },
        if shortcut.shift { "shift+" } else { "" },
        if shortcut.alt { "alt+" } else { "" },
        shortcut.key
    );

    // Enregistrer le nouveau raccourci via le module shortcut
    shortcut::change_binding(app_handle, "ocr".to_string(), shortcut_str)?;

    Ok(())
}

/// Parser un binding string en ClipboardShortcut
fn parse_binding_to_shortcut(binding: &str) -> Result<ClipboardShortcut, String> {
    let parts: Vec<&str> = binding.split('+').collect();

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut meta = false;
    let mut key = String::new();

    for part in &parts {
        match part.trim() {
            "ctrl" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            "super" => meta = true,
            k => key = k.to_string(),
        }
    }

    Ok(ClipboardShortcut {
        ctrl,
        shift,
        alt,
        meta,
        key,
    })
}

#[tauri::command]
fn load_playback_speed() -> Result<f32, String> {
    load_config().map(|cfg| cfg.playback_speed)
}

#[tauri::command]
fn save_playback_speed(speed: f32) -> Result<(), String> {
    update_config(|cfg| cfg.playback_speed = speed.clamp(0.5, 2.0))
}

#[tauri::command]
fn load_dev_mode() -> Result<bool, String> {
    load_config().map(|cfg| cfg.dev_mode)
}

#[tauri::command]
fn save_dev_mode(enabled: bool) -> Result<(), String> {
    update_config(|cfg| cfg.dev_mode = enabled)
}

#[tauri::command]
fn load_source_language() -> Result<String, String> {
    load_config().map(|cfg| cfg.source_language)
}

#[tauri::command]
fn save_source_language(language: String) -> Result<(), String> {
    update_config(|cfg| cfg.source_language = language)
}

#[tauri::command]
fn load_target_language() -> Result<String, String> {
    load_config().map(|cfg| cfg.target_language)
}

#[tauri::command]
fn save_target_language(language: String) -> Result<(), String> {
    update_config(|cfg| cfg.target_language = language)
}

#[tauri::command]
fn reset_shortcuts_to_default() -> Result<(), String> {
    update_config(|cfg| {
        cfg.clipboard_shortcut = ClipboardShortcut::default();
        cfg.ocr_shortcut = ClipboardShortcut {
            ctrl: true,
            shift: false,
            alt: true,
            meta: false,
            key: "o".to_string(),
        };
    })
}

fn get_language_code(detected_lang: DetectedLanguage) -> &'static str {
    match detected_lang {
        DetectedLanguage::English => "en",
        DetectedLanguage::French => "fr",
    }
}

fn translate_if_needed(
    text: &str,
    detected_lang: DetectedLanguage,
    source_lang: &str,
    target_lang: &str,
) -> Result<String, String> {
    let detected_code = get_language_code(detected_lang);

    if source_lang == "auto" {
        if detected_code != target_lang && detected_code == "en" && target_lang == "fr" {
            debug!(
                "[TRANSLATOR] Détection automatique: texte en anglais, traduction en français..."
            );
            match translate(text, "en", "fr") {
                Ok(translated) => {
                    debug!("[TRANSLATOR] Traduction réussie");
                    Ok(translated)
                }
                Err(e) => {
                    debug!("[TRANSLATOR] Erreur de traduction: {}", e);
                    debug!("[TRANSLATOR] Utilisation du texte original");
                    Ok(text.to_string())
                }
            }
        } else {
            Ok(text.to_string())
        }
    } else if source_lang != target_lang {
        debug!(
            "[TRANSLATOR] Traduction de {} en {}...",
            source_lang, target_lang
        );
        match translate(text, source_lang, target_lang) {
            Ok(translated) => {
                debug!("[TRANSLATOR] Traduction réussie");
                Ok(translated)
            }
            Err(e) => {
                debug!("[TRANSLATOR] Erreur de traduction: {}", e);
                debug!("[TRANSLATOR] Utilisation du texte original");
                Ok(text.to_string())
            }
        }
    } else {
        Ok(text.to_string())
    }
}

struct SpeechPipelineInput {
    text: String,
    source_lang: String,
    target_lang: String,
    playback_speed: f32,
    dev_mode: bool,
}

fn execute_speech_pipeline(
    input: SpeechPipelineInput,
    _state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
    log_tag: &str,
) -> Result<(), String> {
    debug!("[{}] Début avec texte: {}", log_tag, input.text);

    let text_to_process = if input.dev_mode {
        debug!(
            "[{}] Mode développeur activé, application de read_vars",
            log_tag
        );
        read_vars(&input.text)
    } else {
        input.text.clone()
    };

    let cleaned_text = preprocess_text(&text_to_process);
    debug!("[{}] Texte nettoyé: {}", log_tag, cleaned_text);

    let detected_lang = detect_language(&cleaned_text);
    debug!("[{}] Langue détectée: {:?}", log_tag, detected_lang);

    let text_to_speak = translate_if_needed(
        &cleaned_text,
        detected_lang,
        &input.source_lang,
        &input.target_lang,
    )?;

    {
        let mut sink_guard = CURRENT_SINK
            .lock()
            .map_err(|e| format!("Erreur lors du verrouillage du sink: {}", e))?;
        if sink_guard.take().is_some() {
            debug!("[{}] Arrêt de la lecture précédente", log_tag);
        }
    }

    debug!("[{}] Création du TTS engine", log_tag);
    #[cfg(target_os = "macos")]
    let mut tts = MacOsTts::new();
    #[cfg(target_os = "linux")]
    let mut tts = EspeakNg::new();
    tts.set_lang(input.target_lang.clone());

    #[cfg(target_os = "macos")]
    let tts_speed = (input.playback_speed * 200.0) as i32;
    #[cfg(target_os = "linux")]
    let tts_speed = ((input.playback_speed * 100.0) as i32).clamp(50, 200);
    tts.set_speed(tts_speed);

    debug!(
        "[{}] Vitesse de lecture: {} (tts_speed: {})",
        log_tag, input.playback_speed, tts_speed
    );
    debug!("[{}] Appel de tts.speak()", log_tag);
    let audio_file_path = tts.speak(&text_to_speak)?;
    debug!("[{}] Fichier audio généré: {}", log_tag, audio_file_path);

    debug!("[{}] Création du stream et sink audio", log_tag);
    let mut device_sink = DeviceSinkBuilder::open_default_sink()
        .map_err(|e| format!("Erreur lors de la création du stream audio: {}", e))?;
    device_sink.log_on_drop(false);
    let player = Player::connect_new(device_sink.mixer());

    debug!("[{}] Lecture du fichier: {}", log_tag, audio_file_path);
    let file = std::fs::File::open(&audio_file_path)
        .map_err(|e| format!("Erreur lors de l'ouverture du fichier: {}", e))?;
    let source = Decoder::new(file)
        .map_err(|e| format!("Erreur lors du décodage du fichier audio: {}", e))?;
    player.append(source);

    let thread_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    {
        let mut sink_guard = CURRENT_SINK
            .lock()
            .map_err(|e| format!("Erreur lors du verrouillage du sink: {}", e))?;
        *sink_guard = Some(AudioPlayback {
            player,
            _device_sink: Box::new(device_sink),
            thread_id,
        });
    }

    {
        let mut current_thread_id = CURRENT_THREAD_ID
            .lock()
            .map_err(|e| format!("Erreur lors du verrouillage du thread_id: {}", e))?;
        *current_thread_id = thread_id;
    }

    let app_handle_clone = app_handle.clone();
    debug!(
        "[{}] Lancement du thread d'attente (ID: {})",
        log_tag, thread_id
    );
    std::thread::spawn(move || {
        debug!("[THREAD#{}] Attente de la fin de la lecture", thread_id);
        let start = std::time::Instant::now();
        let max_duration = std::time::Duration::from_secs(600);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let current_id = CURRENT_THREAD_ID.lock().map(|g| *g).unwrap_or(0);

            if current_id != thread_id {
                debug!(
                    "[THREAD#{}] Ancien thread détecté (nouveau ID: {}), arrêt",
                    thread_id, current_id
                );
                break;
            }

            let sink_guard = match CURRENT_SINK.lock() {
                Ok(g) => g,
                Err(e) => {
                    debug!("[THREAD#{}] Erreur de verrouillage: {}", thread_id, e);
                    break;
                }
            };

            if let Some(playback) = sink_guard.as_ref() {
                if playback.player.empty() {
                    debug!(
                        "[THREAD#{}] Lecture terminée, émission de playback_finished",
                        thread_id
                    );
                    if let Some(window) = app_handle_clone.get_webview_window("main") {
                        let _ = window.emit("playback_finished", ());
                    }
                    break;
                }
            }

            if start.elapsed() > max_duration {
                debug!(
                    "[THREAD#{}] Timeout après 10 minutes, émission de playback_finished",
                    thread_id
                );
                if let Some(window) = app_handle_clone.get_webview_window("main") {
                    let _ = window.emit("playback_finished", ());
                }
                break;
            }
        }
    });

    debug!("[{}] Sink créé et en cours de lecture", log_tag);

    Ok(())
}

#[tauri::command]
fn speak(
    text: String,
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let config = load_config().ok();
    let dev_mode = config.as_ref().map(|c| c.dev_mode).unwrap_or(false);
    let source_lang = config
        .as_ref()
        .map(|c| c.source_language.clone())
        .unwrap_or_else(|| "auto".to_string());
    let target_lang = config
        .as_ref()
        .map(|c| c.target_language.clone())
        .unwrap_or_else(|| "fr".to_string());
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let pipeline = SpeechPipelineInput {
        text,
        source_lang,
        target_lang,
        playback_speed,
        dev_mode,
    };

    execute_speech_pipeline(pipeline, state, app_handle, "SPEAK")
}

#[tauri::command]
fn speak_ocr(state: State<Mutex<PlaybackState>>, app_handle: AppHandle) -> Result<(), String> {
    use std::fs;
    use std::time::SystemTime;

    debug!("[SPEAK_OCR] Début de speak_ocr()");

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Erreur lors de la récupération du timestamp: {}", e))?
        .as_millis();

    #[cfg(target_os = "linux")]
    let screenshot_path =
        std::path::PathBuf::from(format!("/dev/shm/gsp-ui-screenshot-{}.png", timestamp));

    #[cfg(target_os = "macos")]
    let screenshot_path = std::env::temp_dir().join(format!("gsp-ui-screenshot-{}.png", timestamp));

    let screenshot_path_str = screenshot_path
        .to_str()
        .ok_or("Chemin de capture invalide")?
        .to_string();

    debug!("[SPEAK_OCR] Chemin de capture: {}", screenshot_path_str);
    debug!("[SPEAK_OCR] Lancement de la capture d'écran");

    screenshot_region(&screenshot_path_str);

    if !std::path::Path::new(&screenshot_path).exists() {
        debug!("[SPEAK_OCR] Erreur: le fichier de capture n'a pas été créé");
        return Err("La capture d'écran a échoué".to_string());
    }

    debug!("[SPEAK_OCR] Fichier de capture créé");

    let config = load_config().ok();
    let source_lang = config
        .as_ref()
        .map(|c| c.source_language.as_str())
        .unwrap_or("auto");
    let dev_mode = config.as_ref().map(|c| c.dev_mode).unwrap_or(false);

    let tesseract_lang = match source_lang {
        "fr" => "fr-FR",
        "de" => "de-DE",
        "es" => "es-ES",
        "it" => "it-IT",
        "auto" => "auto",
        _ => "en-GB",
    };

    debug!(
        "[SPEAK_OCR] Exécution de Tesseract avec la langue: {}",
        tesseract_lang
    );
    let text = tesseract(&screenshot_path_str, tesseract_lang);

    if text.is_empty() {
        debug!("[SPEAK_OCR] Erreur: Tesseract n'a pas reconnu de texte");
        let _ = fs::remove_file(&screenshot_path);
        return Err("Aucun texte reconnu par OCR".to_string());
    }

    debug!(
        "[SPEAK_OCR] Texte reconnu: {}",
        text.chars().take(50).collect::<String>()
    );

    let _ = fs::remove_file(&screenshot_path);

    let target_lang = config
        .as_ref()
        .map(|c| c.target_language.clone())
        .unwrap_or_else(|| "fr".to_string());
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let pipeline = SpeechPipelineInput {
        text,
        source_lang: source_lang.to_string(),
        target_lang,
        playback_speed,
        dev_mode,
    };

    execute_speech_pipeline(pipeline, state, app_handle, "SPEAK_OCR")
}

#[tauri::command]
fn stop_speak(_state: State<Mutex<PlaybackState>>) -> Result<(), String> {
    debug!("[STOP_SPEAK] Début de stop_speak()");
    let mut sink_guard = CURRENT_SINK
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage du sink: {}", e))?;

    if let Some(playback) = sink_guard.take() {
        debug!("[STOP_SPEAK] Arrêt de la lecture");
        playback.player.stop();
        debug!("[STOP_SPEAK] Lecture arrêtée");
    } else {
        debug!("[STOP_SPEAK] Aucune lecture en cours");
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn get_clipboard_text() -> Result<String, String> {
    use x11_clipboard::Clipboard;

    let clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let atoms = clipboard.getter.atoms.clone();
    let timeout = std::time::Duration::from_secs(1);

    clipboard
        .load(
            clipboard.setter.atoms.primary,
            atoms.utf8_string,
            atoms.property,
            timeout,
        )
        .map_err(|e| format!("Erreur lors de la lecture du presse-papier: {}", e))
        .and_then(|data| {
            String::from_utf8(data).map_err(|e| format!("Erreur de décodage UTF-8: {}", e))
        })
}

#[cfg(target_os = "macos")]
fn get_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let saved = clipboard.get_text().ok();

    thread::sleep(Duration::from_millis(300));

    let output = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to keystroke \"c\" using {command down}",
        ])
        .output()
        .map_err(|e| format!("Erreur d'exécution d'osascript: {}", e))?;

    if !output.status.success() {
        error!(
            "osascript a échoué: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    thread::sleep(Duration::from_millis(100));

    let text = clipboard
        .get_text()
        .map_err(|e| format!("Erreur lors de la lecture de la sélection: {}", e))?;

    if saved.as_ref() == Some(&text) {
        return Err("Aucun texte sélectionné".to_string());
    }

    if let Some(ref saved_text) = saved {
        let _ = clipboard.set_text(saved_text);
    }

    Ok(text)
}

#[tauri::command]
fn speak_clipboard(
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    debug!("[SPEAK_CLIPBOARD] Début de speak_clipboard()");

    let text = match get_clipboard_text() {
        Ok(t) => t,
        Err(e) => {
            error!("[SPEAK_CLIPBOARD] Erreur get_clipboard_text: {}", e);
            return Err(e);
        }
    };

    debug!(
        "[SPEAK_CLIPBOARD] Texte récupéré du presse-papier: {}",
        text.chars().take(50).collect::<String>()
    );

    let config = load_config().ok();
    let dev_mode = config.as_ref().map(|c| c.dev_mode).unwrap_or(false);
    let source_lang = config
        .as_ref()
        .map(|c| c.source_language.clone())
        .unwrap_or_else(|| "auto".to_string());
    let target_lang = config
        .as_ref()
        .map(|c| c.target_language.clone())
        .unwrap_or_else(|| "fr".to_string());
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let pipeline = SpeechPipelineInput {
        text,
        source_lang,
        target_lang,
        playback_speed,
        dev_mode,
    };

    execute_speech_pipeline(pipeline, state, app_handle, "SPEAK_CLIPBOARD")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(PlaybackState { _dummy: 0 }))
        .setup(|app| {
            // Initialiser le moteur de traduction en arrière-plan (non-bloquant)
            std::thread::spawn(|| {
                info!("[SETUP] Initialisation du moteur de traduction en arrière-plan...");
                match translator::initialize_engine() {
                    Ok(_) => info!("[SETUP] Moteur de traduction initialisé avec succès"),
                    Err(e) => info!("[SETUP] Moteur de traduction non disponible: {}", e),
                }
            });

            if let Err(e) = setup_tray(app) {
                error!("Erreur lors de la création de la tray-icon: {}", e);
            }

            if let Err(e) = setup_global_shortcut(app) {
                error!("Erreur lors de la configuration du raccourci global: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_shortcut_config,
            save_shortcut_config,
            load_ocr_shortcut_config,
            save_ocr_shortcut_config,
            load_playback_speed,
            save_playback_speed,
            load_dev_mode,
            save_dev_mode,
            load_source_language,
            save_source_language,
            load_target_language,
            save_target_language,
            reset_shortcuts_to_default,
            shortcut::get_binding,
            shortcut::get_all_bindings,
            shortcut::change_binding,
            shortcut::reset_binding,
            shortcut::suspend_binding,
            shortcut::resume_binding,
            shortcut::format_shortcut_binding,
            speak,
            stop_speak,
            speak_clipboard,
            speak_ocr
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_binding_to_shortcut_simple_key() {
        // Teste le parsing d'une touche simple sans modificateurs
        let result = parse_binding_to_shortcut("v").unwrap();
        assert!(!result.ctrl);
        assert!(!result.shift);
        assert!(!result.alt);
        assert!(!result.meta);
        assert_eq!(result.key, "v");
    }

    #[test]
    fn test_parse_binding_to_shortcut_with_modifiers() {
        // Teste le parsing avec plusieurs modificateurs
        let result = parse_binding_to_shortcut("ctrl+shift+v").unwrap();
        assert!(result.ctrl);
        assert!(result.shift);
        assert!(!result.alt);
        assert!(!result.meta);
        assert_eq!(result.key, "v");
    }

    #[test]
    fn test_parse_binding_to_shortcut_with_alt_and_super() {
        // Teste alt et super (meta)
        let result = parse_binding_to_shortcut("alt+super+o").unwrap();
        assert!(!result.ctrl);
        assert!(!result.shift);
        assert!(result.alt);
        assert!(result.meta);
        assert_eq!(result.key, "o");
    }

    #[test]
    fn test_parse_binding_to_shortcut_preserves_key_case() {
        // Teste que la casse de la touche est préservée
        let result = parse_binding_to_shortcut("ctrl+V").unwrap();
        assert_eq!(result.key, "V");
    }

    #[test]
    fn test_parse_binding_to_shortcut_with_spaces() {
        // Teste le parsing avec espaces autour des délimiteurs
        let result = parse_binding_to_shortcut("ctrl+ shift +alt +v").unwrap();
        assert!(result.ctrl);
        assert!(result.shift);
        assert!(result.alt);
        assert_eq!(result.key, "v");
    }

    #[test]
    fn test_get_language_code_english() {
        // Teste la conversion de DetectedLanguage::English
        let code = get_language_code(DetectedLanguage::English);
        assert_eq!(code, "en");
    }

    #[test]
    fn test_get_language_code_french() {
        // Teste la conversion de DetectedLanguage::French
        let code = get_language_code(DetectedLanguage::French);
        assert_eq!(code, "fr");
    }

    #[test]
    fn test_clipboard_shortcut_default() {
        // Teste les valeurs par défaut du raccourci
        let default = ClipboardShortcut::default();
        assert!(default.ctrl);
        assert!(default.shift);
        assert!(!default.alt);
        assert!(!default.meta);
        assert_eq!(default.key, "V");
    }

    #[test]
    fn test_app_config_default() {
        // Teste les valeurs par défaut de la config
        let config = AppConfig::default();
        assert_eq!(config.playback_speed, 1.0);
        assert!(!config.dev_mode);
        assert_eq!(config.source_language, "auto");
        assert_eq!(config.target_language, "fr");

        // Vérifie la config du raccourci clipboard
        assert!(config.clipboard_shortcut.ctrl);
        assert!(config.clipboard_shortcut.shift);
        assert_eq!(config.clipboard_shortcut.key, "V");
    }

    #[test]
    fn test_translate_if_needed_source_language_same_as_target() {
        // Quand source_lang == target_lang, le texte ne devrait pas être traduit
        let result = translate_if_needed("Hello", DetectedLanguage::English, "en", "en");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn test_translate_if_needed_auto_detection_same_language() {
        // En mode auto, si la langue détectée == langue cible, pas de traduction
        let result = translate_if_needed("Bonjour", DetectedLanguage::French, "auto", "fr");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Bonjour");
    }

    #[test]
    fn test_translate_if_needed_auto_detection_different_language_english_to_french() {
        // En mode auto avec détection anglais → français: devrait tenter une traduction
        // Note: Ce test dépend de la vraie fonction translate(),
        // donc il teste le chemin logique, pas le résultat exact
        let result = translate_if_needed("Hello", DetectedLanguage::English, "auto", "fr");
        assert!(result.is_ok());
        // Le résultat devrait être soit traduit, soit le texte original en cas d'erreur
        let text = result.unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_parse_binding_to_shortcut_multiple_modifiers_order() {
        // Teste que l'ordre des modificateurs n'a pas d'importance
        let result1 = parse_binding_to_shortcut("ctrl+alt+shift+x").unwrap();
        let result2 = parse_binding_to_shortcut("shift+ctrl+alt+x").unwrap();

        assert_eq!(result1.ctrl, result2.ctrl);
        assert_eq!(result1.alt, result2.alt);
        assert_eq!(result1.shift, result2.shift);
        assert_eq!(result1.key, result2.key);
    }
}
