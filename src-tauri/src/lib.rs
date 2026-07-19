mod language_detector;
mod ocr;
mod screenshooter;
mod textutils;
mod translator;
mod tts;

use language_detector::{detect_language, DetectedLanguage};
use ocr::tesseract;
use serde::{Deserialize, Serialize};
use screenshooter::xfce4_screenshooter_region;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use textutils::{preprocess_text, read_vars};
use translator::translate;
use tts::{EspeakNg, TtsEngine};

macro_rules! debug {
    ($tag:expr, $($arg:tt)*) => {
        eprintln!("[{}] {}", $tag, format!($($arg)*))
    };
}

struct PlaybackState {
    child_pid: Option<u32>,
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
            key: "c".to_string(),
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
                key: "c".to_string(),
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
    let config_path = get_config_path()?;

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Erreur de lecture du fichier de config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Erreur de parsing du fichier de config: {}", e))
    } else {
        Ok(AppConfig::default())
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Erreur de sérialisation: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Erreur d'écriture du fichier de config: {}", e))
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

fn shortcut_to_string(shortcut: &ClipboardShortcut) -> String {
    let mut keys = vec![];
    if shortcut.ctrl {
        keys.push("ctrl");
    }
    if shortcut.shift {
        keys.push("shift");
    }
    if shortcut.alt {
        keys.push("alt");
    }
    if shortcut.meta {
        keys.push("meta");
    }
    keys.push(&shortcut.key);
    keys.join("+")
}

fn register_shortcut_internal(
    app_handle: &AppHandle,
    shortcut_str: &str,
    is_ocr: bool,
    log_prefix: &str,
) -> Result<(), String> {
    let shortcut = shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci: {:?}", e))?;

    debug!(log_prefix, "Tentative de désenregistrement du raccourci précédent");

    let unregister_result = app_handle.global_shortcut().unregister(shortcut);
    debug!(log_prefix, "Résultat du désenregistrement: {:?}", unregister_result);

    debug!(log_prefix, "Enregistrement du raccourci");
    app_handle
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _accelerator, _state| {
            let event_name = if is_ocr {
                "global_shortcut_ocr_triggered"
            } else {
                "global_shortcut_triggered"
            };
            debug!("CALLBACK", "Raccourci déclenché!");
            if let Some(window) = app.get_webview_window("main") {
                debug!("CALLBACK", "Émission de {}", event_name);
                let _ = window.emit(event_name, ());
            }
        })
        .map_err(|e| format!("Erreur lors de l'enregistrement: {:?}", e))?;

    debug!(log_prefix, "Raccourci enregistré avec succès");
    Ok(())
}

fn setup_global_shortcut(app: &tauri::App) -> Result<(), String> {
    debug!("SETUP", "Début de setup_global_shortcut()");
    let app_handle = app.handle().clone();

    if let Ok(config) = load_config() {
        let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);
        debug!("SETUP", "Raccourci clipboard à enregistrer: {}", shortcut_str);

        register_shortcut_internal(&app_handle, &shortcut_str, false, "SETUP")?;

        let ocr_shortcut_str = shortcut_to_string(&config.ocr_shortcut);
        debug!("SETUP", "Raccourci OCR à enregistrer: {}", ocr_shortcut_str);

        register_shortcut_internal(&app_handle, &ocr_shortcut_str, true, "SETUP")
    } else {
        debug!("SETUP", "Impossible de charger la configuration");
        Err("Impossible de charger la configuration".to_string())
    }
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
    load_config().map(|cfg| cfg.clipboard_shortcut)
}

#[tauri::command]
fn save_shortcut_config(shortcut: ClipboardShortcut) -> Result<(), String> {
    update_config(|cfg| cfg.clipboard_shortcut = shortcut)
}

#[tauri::command]
fn load_ocr_shortcut_config() -> Result<ClipboardShortcut, String> {
    load_config().map(|cfg| cfg.ocr_shortcut)
}

#[tauri::command]
fn save_ocr_shortcut_config(shortcut: ClipboardShortcut) -> Result<(), String> {
    update_config(|cfg| cfg.ocr_shortcut = shortcut)
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
            debug!("TRANSLATOR", "Détection automatique: texte en anglais, traduction en français...");
            match translate(text, "en", "fr") {
                Ok(translated) => {
                    debug!("TRANSLATOR", "Traduction réussie");
                    Ok(translated)
                }
                Err(e) => {
                    debug!("TRANSLATOR", "Erreur de traduction: {}", e);
                    debug!("TRANSLATOR", "Utilisation du texte original");
                    Ok(text.to_string())
                }
            }
        } else {
            Ok(text.to_string())
        }
    } else if source_lang != target_lang {
        debug!("TRANSLATOR", "Traduction de {} en {}...", source_lang, target_lang);
        match translate(text, source_lang, target_lang) {
            Ok(translated) => {
                debug!("TRANSLATOR", "Traduction réussie");
                Ok(translated)
            }
            Err(e) => {
                debug!("TRANSLATOR", "Erreur de traduction: {}", e);
                debug!("TRANSLATOR", "Utilisation du texte original");
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
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
    log_tag: &str,
) -> Result<(), String> {
    debug!(log_tag, "Début avec texte: {}", input.text);

    let text_to_process = if input.dev_mode {
        debug!(log_tag, "Mode développeur activé, application de read_vars");
        read_vars(&input.text)
    } else {
        input.text.clone()
    };

    let cleaned_text = preprocess_text(&text_to_process);
    debug!(log_tag, "Texte nettoyé: {}", cleaned_text);

    let detected_lang = detect_language(&cleaned_text);
    debug!(log_tag, "Langue détectée: {:?}", detected_lang);

    let text_to_speak =
        translate_if_needed(&cleaned_text, detected_lang, &input.source_lang, &input.target_lang)?;

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        debug!(log_tag, "Arrêt du processus précédent (PID: {})", pid);
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }

    drop(playback);

    debug!(log_tag, "Création du TTS engine");
    let mut tts = EspeakNg::new();
    tts.set_lang(input.target_lang.clone());

    let espeak_speed = ((input.playback_speed * 100.0) as i32).clamp(50, 200);
    tts.set_speed(espeak_speed);

    debug!(log_tag, "Vitesse de lecture: {} (espeak: {})", input.playback_speed, espeak_speed);
    debug!(log_tag, "Appel de tts.speak()");
    let mut child = tts.speak(&text_to_speak)?;
    let pid = child.id();
    debug!(log_tag, "Child lancé avec PID: {}", pid);

    let app_handle_clone = app_handle.clone();
    debug!(log_tag, "Lancement du thread d'attente");
    std::thread::spawn(move || {
        debug!("THREAD", "Attente du processus PID: {}", pid);
        let _ = child.wait();
        debug!("THREAD", "Processus terminé, émission de playback_finished");
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.emit("playback_finished", ());
        }
    });

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;
    playback.child_pid = Some(pid);
    debug!(log_tag, "PID {} stocké dans state", pid);

    Ok(())
}

#[tauri::command]
fn register_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    debug!("REGISTER", "Début de register_global_shortcut()");
    let config = load_config()?;

    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);
    debug!("REGISTER", "Raccourci clipboard à enregistrer: {}", shortcut_str);
    register_shortcut_internal(&app_handle, &shortcut_str, false, "REGISTER")?;

    let ocr_shortcut_str = shortcut_to_string(&config.ocr_shortcut);
    debug!("REGISTER", "Raccourci OCR à enregistrer: {}", ocr_shortcut_str);
    register_shortcut_internal(&app_handle, &ocr_shortcut_str, true, "REGISTER")
}

#[tauri::command]
fn unregister_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    let config = load_config()?;
    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);

    let shortcut = shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci: {:?}", e))?;

    app_handle
        .global_shortcut()
        .unregister(shortcut)
        .map_err(|e| format!("Erreur lors du désenregistrement du raccourci: {}", e))
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
fn speak_ocr(
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use std::fs;
    use std::time::SystemTime;

    debug!("SPEAK_OCR", "Début de speak_ocr()");

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Erreur lors de la récupération du timestamp: {}", e))?
        .as_millis();

    let screenshot_path = format!("/dev/shm/gsp-ui-screenshot-{}.png", timestamp);
    let screenshot_path_str = screenshot_path.as_str();

    debug!("SPEAK_OCR", "Chemin de capture: {}", screenshot_path_str);
    debug!("SPEAK_OCR", "Lancement de xfce4-screenshooter");

    xfce4_screenshooter_region(screenshot_path_str);

    if !std::path::Path::new(&screenshot_path).exists() {
        debug!("SPEAK_OCR", "Erreur: le fichier de capture n'a pas été créé");
        return Err("La capture d'écran a échoué".to_string());
    }

    debug!("SPEAK_OCR", "Fichier de capture créé");

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

    debug!("SPEAK_OCR", "Exécution de Tesseract avec la langue: {}", tesseract_lang);
    let text = tesseract(screenshot_path_str, tesseract_lang);

    if text.is_empty() {
        debug!("SPEAK_OCR", "Erreur: Tesseract n'a pas reconnu de texte");
        let _ = fs::remove_file(&screenshot_path);
        return Err("Aucun texte reconnu par OCR".to_string());
    }

    debug!("SPEAK_OCR", "Texte reconnu: {}", text.chars().take(50).collect::<String>());

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
fn stop_speak(state: State<Mutex<PlaybackState>>) -> Result<(), String> {
    debug!("STOP_SPEAK", "Début de stop_speak()");
    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        debug!("STOP_SPEAK", "Arrêt du processus PID: {}", pid);
        let kill_result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        debug!("STOP_SPEAK", "Résultat de kill: {}", kill_result);
    } else {
        debug!("STOP_SPEAK", "Aucun processus à arrêter");
    }

    Ok(())
}

#[tauri::command]
fn speak_clipboard(
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use x11_clipboard::Clipboard;

    debug!("SPEAK_CLIPBOARD", "Début de speak_clipboard()");

    let clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let atoms = clipboard.getter.atoms.clone();
    let timeout = std::time::Duration::from_secs(1);

    let text = clipboard
        .load(
            clipboard.setter.atoms.primary,
            atoms.utf8_string,
            atoms.property,
            timeout,
        )
        .map_err(|e| format!("Erreur lors de la lecture du presse-papier: {}", e))
        .and_then(|data| {
            String::from_utf8(data).map_err(|e| format!("Erreur de décodage UTF-8: {}", e))
        })?;

    debug!("SPEAK_CLIPBOARD", "Texte récupéré du presse-papier: {}", text.chars().take(50).collect::<String>());

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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(PlaybackState { child_pid: None }))
        .setup(|app| {
            if let Err(e) = setup_tray(app) {
                eprintln!("Erreur lors de la création de la tray-icon: {}", e);
            }

            if let Err(e) = setup_global_shortcut(app) {
                eprintln!("Erreur lors de la configuration du raccourci global: {}", e);
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
            register_global_shortcut,
            unregister_global_shortcut,
            load_playback_speed,
            save_playback_speed,
            load_dev_mode,
            save_dev_mode,
            load_source_language,
            save_source_language,
            load_target_language,
            save_target_language,
            speak,
            stop_speak,
            speak_clipboard,
            speak_ocr
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
