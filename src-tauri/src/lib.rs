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
use textutils::preprocess_text;
use translator::translate;
use tts::{EspeakNg, TtsEngine};

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
    eprintln!("[SETUP] Début de setup_global_shortcut()");
    let app_handle = app.handle().clone();

    if let Ok(config) = load_config() {
        // Enregistrer le raccourci clipboard
        let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);
        eprintln!("[SETUP] Raccourci clipboard à enregistrer: {}", shortcut_str);

        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                eprintln!("[SETUP] Parsing réussi, enregistrement du raccourci clipboard");
                if let Err(e) = app_handle.global_shortcut().on_shortcut(shortcut, move |app, _accelerator, _state| {
                    eprintln!("[SHORTCUT CALLBACK] Raccourci clipboard déclenché!");
                    if let Some(window) = app.get_webview_window("main") {
                        eprintln!("[SHORTCUT CALLBACK] Fenêtre trouvée, émission de global_shortcut_triggered");
                        let _ = window.emit("global_shortcut_triggered", ());
                    }
                }) {
                    eprintln!("[SETUP] Erreur lors de l'enregistrement du raccourci clipboard: {:?}", e);
                    return Err(format!("Erreur lors de l'enregistrement du raccourci clipboard: {:?}", e));
                }
                eprintln!(
                    "[SETUP] Raccourci clipboard enregistré avec succès: {}",
                    shortcut_str
                );
            }
            Err(e) => {
                eprintln!(
                    "[SETUP] Erreur lors du parsing du raccourci clipboard {}: {:?}",
                    shortcut_str, e
                );
                return Err(format!("Erreur lors du parsing du raccourci clipboard: {:?}", e));
            }
        }

        // Enregistrer le raccourci OCR
        let ocr_shortcut_str = shortcut_to_string(&config.ocr_shortcut);
        eprintln!("[SETUP] Raccourci OCR à enregistrer: {}", ocr_shortcut_str);

        match ocr_shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                eprintln!("[SETUP] Parsing réussi, enregistrement du raccourci OCR");
                if let Err(e) = app_handle.global_shortcut().on_shortcut(shortcut, move |app, _accelerator, _state| {
                    eprintln!("[SHORTCUT CALLBACK] Raccourci OCR déclenché!");
                    if let Some(window) = app.get_webview_window("main") {
                        eprintln!("[SHORTCUT CALLBACK] Fenêtre trouvée, émission de global_shortcut_ocr_triggered");
                        let _ = window.emit("global_shortcut_ocr_triggered", ());
                    }
                }) {
                    eprintln!("[SETUP] Erreur lors de l'enregistrement du raccourci OCR: {:?}", e);
                    return Err(format!("Erreur lors de l'enregistrement du raccourci OCR: {:?}", e));
                }
                eprintln!(
                    "[SETUP] Raccourci OCR enregistré avec succès: {}",
                    ocr_shortcut_str
                );
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "[SETUP] Erreur lors du parsing du raccourci OCR {}: {:?}",
                    ocr_shortcut_str, e
                );
                Err(format!("Erreur lors du parsing du raccourci OCR: {:?}", e))
            }
        }
    } else {
        eprintln!("[SETUP] Impossible de charger la configuration");
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
    let mut config = load_config()?;
    config.clipboard_shortcut = shortcut.clone();
    save_config(&config)
}

#[tauri::command]
fn load_ocr_shortcut_config() -> Result<ClipboardShortcut, String> {
    load_config().map(|cfg| cfg.ocr_shortcut)
}

#[tauri::command]
fn save_ocr_shortcut_config(shortcut: ClipboardShortcut) -> Result<(), String> {
    let mut config = load_config()?;
    config.ocr_shortcut = shortcut.clone();
    save_config(&config)
}

#[tauri::command]
fn load_playback_speed() -> Result<f32, String> {
    load_config().map(|cfg| cfg.playback_speed)
}

#[tauri::command]
fn save_playback_speed(speed: f32) -> Result<(), String> {
    let mut config = load_config()?;
    config.playback_speed = speed.clamp(0.5, 2.0);
    save_config(&config)
}

#[tauri::command]
fn load_dev_mode() -> Result<bool, String> {
    load_config().map(|cfg| cfg.dev_mode)
}

#[tauri::command]
fn save_dev_mode(enabled: bool) -> Result<(), String> {
    let mut config = load_config()?;
    config.dev_mode = enabled;
    save_config(&config)
}

#[tauri::command]
fn load_source_language() -> Result<String, String> {
    load_config().map(|cfg| cfg.source_language)
}

#[tauri::command]
fn save_source_language(language: String) -> Result<(), String> {
    let mut config = load_config()?;
    config.source_language = language;
    save_config(&config)
}

#[tauri::command]
fn load_target_language() -> Result<String, String> {
    load_config().map(|cfg| cfg.target_language)
}

#[tauri::command]
fn save_target_language(language: String) -> Result<(), String> {
    let mut config = load_config()?;
    config.target_language = language;
    save_config(&config)
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
            eprintln!("[TRANSLATOR] Détection automatique: texte en anglais, traduction en français...");
            match translate(text, "en", "fr") {
                Ok(translated) => {
                    eprintln!("[TRANSLATOR] Traduction réussie");
                    Ok(translated)
                }
                Err(e) => {
                    eprintln!("[TRANSLATOR] Erreur de traduction: {}", e);
                    eprintln!("[TRANSLATOR] Utilisation du texte original");
                    Ok(text.to_string())
                }
            }
        } else {
            Ok(text.to_string())
        }
    } else if source_lang != target_lang {
        eprintln!(
            "[TRANSLATOR] Traduction de {} en {}...",
            source_lang, target_lang
        );
        match translate(text, source_lang, target_lang) {
            Ok(translated) => {
                eprintln!("[TRANSLATOR] Traduction réussie");
                Ok(translated)
            }
            Err(e) => {
                eprintln!("[TRANSLATOR] Erreur de traduction: {}", e);
                eprintln!("[TRANSLATOR] Utilisation du texte original");
                Ok(text.to_string())
            }
        }
    } else {
        Ok(text.to_string())
    }
}

#[tauri::command]
fn register_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    eprintln!("[REGISTER] Début de register_global_shortcut()");
    let config = load_config()?;

    // Enregistrer le raccourci clipboard
    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);
    let shortcut = shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci clipboard: {:?}", e))?;

    eprintln!("[REGISTER] Raccourci clipboard à enregistrer: {}", shortcut_str);
    eprintln!("[REGISTER] Tentative de désenregistrement du raccourci clipboard précédent");

    let unregister_result = app_handle.global_shortcut().unregister(shortcut);
    eprintln!(
        "[REGISTER] Résultat du désenregistrement clipboard: {:?}",
        unregister_result
    );

    eprintln!("[REGISTER] Enregistrement du raccourci clipboard");
    app_handle
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _accelerator, _state| {
            eprintln!("[REGISTER CALLBACK] Raccourci clipboard déclenché!");
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("[REGISTER CALLBACK] Émission de global_shortcut_triggered");
                let _ = window.emit("global_shortcut_triggered", ());
            }
        })
        .map_err(|e| {
            format!(
                "Erreur lors de l'enregistrement du raccourci clipboard {}: {:?}",
                shortcut_str, e
            )
        })?;

    // Enregistrer le raccourci OCR
    let ocr_shortcut_str = shortcut_to_string(&config.ocr_shortcut);
    let ocr_shortcut = ocr_shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci OCR: {:?}", e))?;

    eprintln!("[REGISTER] Raccourci OCR à enregistrer: {}", ocr_shortcut_str);
    eprintln!("[REGISTER] Tentative de désenregistrement du raccourci OCR précédent");

    let unregister_result = app_handle.global_shortcut().unregister(ocr_shortcut);
    eprintln!(
        "[REGISTER] Résultat du désenregistrement OCR: {:?}",
        unregister_result
    );

    eprintln!("[REGISTER] Enregistrement du raccourci OCR");
    app_handle
        .global_shortcut()
        .on_shortcut(ocr_shortcut, move |app, _accelerator, _state| {
            eprintln!("[REGISTER CALLBACK] Raccourci OCR déclenché!");
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("[REGISTER CALLBACK] Émission de global_shortcut_ocr_triggered");
                let _ = window.emit("global_shortcut_ocr_triggered", ());
            }
        })
        .map_err(|e| {
            format!(
                "Erreur lors de l'enregistrement du raccourci OCR {}: {:?}",
                ocr_shortcut_str, e
            )
        })
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
    eprintln!("[SPEAK] Début de speak() avec texte: {}", text);

    let cleaned_text = preprocess_text(&text);
    eprintln!("[SPEAK] Texte nettoyé: {}", cleaned_text);

    let detected_lang = detect_language(&cleaned_text);
    eprintln!("[SPEAK] Langue détectée: {:?}", detected_lang);

    let config = load_config().ok();
    let source_lang = config.as_ref().map(|c| c.source_language.as_str()).unwrap_or("auto");
    let target_lang = config.as_ref().map(|c| c.target_language.as_str()).unwrap_or("fr");
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let text_to_speak = translate_if_needed(&cleaned_text, detected_lang, source_lang, target_lang)?;

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        eprintln!("[SPEAK] Arrêt du processus précédent (PID: {})", pid);
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }

    drop(playback);

    eprintln!("[SPEAK] Création du TTS engine");
    let mut tts = EspeakNg::new();
    tts.set_lang(target_lang.to_string());

    // Convertir le multiplicateur (1.0-2.0) en valeur eSpeak (50-200)
    let espeak_speed = ((playback_speed * 100.0) as i32).clamp(50, 200);
    tts.set_speed(espeak_speed);

    eprintln!(
        "[SPEAK] Vitesse de lecture: {} (espeak: {})",
        playback_speed, espeak_speed
    );
    eprintln!("[SPEAK] Appel de tts.speak()");
    let mut child = tts.speak(&text_to_speak)?;
    let pid = child.id();
    eprintln!("[SPEAK] Child lancé avec PID: {}", pid);

    let app_handle_clone = app_handle.clone();
    eprintln!("[SPEAK] Lancement du thread d'attente");
    std::thread::spawn(move || {
        eprintln!("[THREAD] Attente du processus PID: {}", pid);
        let _ = child.wait();
        eprintln!("[THREAD] Processus terminé, émission de playback_finished");
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.emit("playback_finished", ());
        }
    });

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;
    playback.child_pid = Some(pid);
    eprintln!("[SPEAK] PID {} stocké dans state", pid);

    Ok(())
}

#[tauri::command]
fn speak_ocr(
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use std::fs;
    use std::time::SystemTime;

    eprintln!("[SPEAK_OCR] Début de speak_ocr()");

    let temp_dir = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Erreur lors de la récupération du timestamp: {}", e))?
        .as_millis();

    let screenshot_path = temp_dir.join(format!("gsp-ui-screenshot-{}.png", timestamp));
    let screenshot_path_str = screenshot_path
        .to_str()
        .ok_or_else(|| "Impossible de convertir le chemin en string".to_string())?;

    eprintln!("[SPEAK_OCR] Chemin de capture: {}", screenshot_path_str);
    eprintln!("[SPEAK_OCR] Lancement de xfce4-screenshooter");

    xfce4_screenshooter_region(screenshot_path_str);

    eprintln!("[SPEAK_OCR] Attente de la création du fichier");
    std::thread::sleep(std::time::Duration::from_secs(2));

    if !screenshot_path.exists() {
        eprintln!("[SPEAK_OCR] Erreur: le fichier de capture n'a pas été créé");
        return Err("La capture d'écran a échoué".to_string());
    }

    eprintln!("[SPEAK_OCR] Fichier de capture créé");

    let config = load_config().ok();
    let source_lang = config.as_ref().map(|c| c.source_language.as_str()).unwrap_or("auto");

    let tesseract_lang = match source_lang {
        "auto" | "en" => "en-GB",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "es" => "es-ES",
        "it" => "it-IT",
        _ => "en-GB",
    };

    eprintln!("[SPEAK_OCR] Exécution de Tesseract avec la langue: {}", tesseract_lang);
    let text = tesseract(screenshot_path_str, tesseract_lang);

    if text.is_empty() {
        eprintln!("[SPEAK_OCR] Erreur: Tesseract n'a pas reconnu de texte");
        let _ = fs::remove_file(&screenshot_path);
        return Err("Aucun texte reconnu par OCR".to_string());
    }

    eprintln!(
        "[SPEAK_OCR] Texte reconnu: {}",
        text.chars().take(50).collect::<String>()
    );

    let _ = fs::remove_file(&screenshot_path);

    let cleaned_text = preprocess_text(&text);
    eprintln!("[SPEAK_OCR] Texte nettoyé: {}", cleaned_text);

    let detected_lang = detect_language(&cleaned_text);
    eprintln!("[SPEAK_OCR] Langue détectée: {:?}", detected_lang);

    let target_lang = config.as_ref().map(|c| c.target_language.as_str()).unwrap_or("fr");
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let text_to_speak = translate_if_needed(&cleaned_text, detected_lang, source_lang, target_lang)?;

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        eprintln!("[SPEAK_OCR] Arrêt du processus précédent (PID: {})", pid);
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }

    drop(playback);

    eprintln!("[SPEAK_OCR] Création du TTS engine");
    let mut tts = EspeakNg::new();
    tts.set_lang(target_lang.to_string());

    let espeak_speed = ((playback_speed * 100.0) as i32).clamp(50, 200);
    tts.set_speed(espeak_speed);

    eprintln!(
        "[SPEAK_OCR] Vitesse de lecture: {} (espeak: {})",
        playback_speed, espeak_speed
    );
    eprintln!("[SPEAK_OCR] Appel de tts.speak()");
    let mut child = tts.speak(&text_to_speak)?;
    let pid = child.id();
    eprintln!("[SPEAK_OCR] Child lancé avec PID: {}", pid);

    let app_handle_clone = app_handle.clone();
    eprintln!("[SPEAK_OCR] Lancement du thread d'attente");
    std::thread::spawn(move || {
        eprintln!("[THREAD] Attente du processus PID: {}", pid);
        let _ = child.wait();
        eprintln!("[THREAD] Processus terminé, émission de playback_finished");
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.emit("playback_finished", ());
        }
    });

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;
    playback.child_pid = Some(pid);
    eprintln!("[SPEAK_OCR] PID {} stocké dans state", pid);

    Ok(())
}

#[tauri::command]
fn stop_speak(state: State<Mutex<PlaybackState>>) -> Result<(), String> {
    eprintln!("[STOP_SPEAK] Début de stop_speak()");
    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        eprintln!("[STOP_SPEAK] Arrêt du processus PID: {}", pid);
        let kill_result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        eprintln!("[STOP_SPEAK] Résultat de kill: {}", kill_result);
    } else {
        eprintln!("[STOP_SPEAK] Aucun processus à arrêter");
    }

    Ok(())
}

#[tauri::command]
fn speak_clipboard(
    state: State<Mutex<PlaybackState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use x11_clipboard::Clipboard;

    eprintln!("[SPEAK_CLIPBOARD] Début de speak_clipboard()");

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

    eprintln!(
        "[SPEAK_CLIPBOARD] Texte récupéré du presse-papier: {}",
        text.chars().take(50).collect::<String>()
    );

    let cleaned_text = preprocess_text(&text);
    eprintln!("[SPEAK_CLIPBOARD] Texte nettoyé: {}", cleaned_text);

    let detected_lang = detect_language(&cleaned_text);
    eprintln!("[SPEAK_CLIPBOARD] Langue détectée: {:?}", detected_lang);

    let config = load_config().ok();
    let source_lang = config.as_ref().map(|c| c.source_language.as_str()).unwrap_or("auto");
    let target_lang = config.as_ref().map(|c| c.target_language.as_str()).unwrap_or("fr");
    let playback_speed = config.as_ref().map(|c| c.playback_speed).unwrap_or(1.0);

    let text_to_speak = translate_if_needed(&cleaned_text, detected_lang, source_lang, target_lang)?;

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;

    if let Some(pid) = playback.child_pid.take() {
        eprintln!(
            "[SPEAK_CLIPBOARD] Arrêt du processus précédent (PID: {})",
            pid
        );
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }

    drop(playback);

    eprintln!("[SPEAK_CLIPBOARD] Création du TTS engine");
    let mut tts = EspeakNg::new();
    tts.set_lang(target_lang.to_string());

    let espeak_speed = ((playback_speed * 100.0) as i32).clamp(50, 200);
    tts.set_speed(espeak_speed);

    eprintln!(
        "[SPEAK_CLIPBOARD] Vitesse de lecture: {} (espeak: {})",
        playback_speed, espeak_speed
    );
    eprintln!("[SPEAK_CLIPBOARD] Appel de tts.speak()");
    let mut child = tts.speak(&text_to_speak)?;
    let pid = child.id();
    eprintln!("[SPEAK_CLIPBOARD] Child lancé avec PID: {}", pid);

    let app_handle_clone = app_handle.clone();
    eprintln!("[SPEAK_CLIPBOARD] Lancement du thread d'attente");
    std::thread::spawn(move || {
        eprintln!("[THREAD] Attente du processus PID: {}", pid);
        let _ = child.wait();
        eprintln!("[THREAD] Processus terminé, émission de playback_finished");
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.emit("playback_finished", ());
        }
    });

    let mut playback = state
        .lock()
        .map_err(|e| format!("Erreur lors du verrouillage de l'état: {}", e))?;
    playback.child_pid = Some(pid);
    eprintln!("[SPEAK_CLIPBOARD] PID {} stocké dans state", pid);

    Ok(())
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
