mod language_detector;
mod textutils;
mod translator;
mod tts;

use language_detector::{detect_language, DetectedLanguage};
use serde::{Deserialize, Serialize};
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
    playback_speed: f32,
    #[serde(default)]
    dev_mode: bool,
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
            playback_speed: 1.0,
            dev_mode: false,
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
        let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);
        eprintln!("[SETUP] Raccourci à enregistrer: {}", shortcut_str);

        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                eprintln!("[SETUP] Parsing réussi, enregistrement du raccourci");
                if let Err(e) = app_handle.global_shortcut().on_shortcut(shortcut, move |app, _accelerator, _state| {
                    eprintln!("[SHORTCUT CALLBACK] Raccourci global déclenché!");
                    if let Some(window) = app.get_webview_window("main") {
                        eprintln!("[SHORTCUT CALLBACK] Fenêtre trouvée, émission de global_shortcut_triggered");
                        let _ = window.emit("global_shortcut_triggered", ());
                    }
                }) {
                    eprintln!("[SETUP] Erreur lors de l'enregistrement du raccourci: {:?}", e);
                    return Err(format!("Erreur lors de l'enregistrement du raccourci: {:?}", e));
                }
                eprintln!(
                    "[SETUP] Raccourci global enregistré avec succès: {}",
                    shortcut_str
                );
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "[SETUP] Erreur lors du parsing du raccourci {}: {:?}",
                    shortcut_str, e
                );
                Err(format!("Erreur lors du parsing du raccourci: {:?}", e))
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

/// Traduit le texte en français si la langue détectée est l'anglais
fn translate_to_french_if_english(
    text: &str,
    detected_lang: DetectedLanguage,
) -> Result<String, String> {
    match detected_lang {
        DetectedLanguage::English => {
            eprintln!("[TRANSLATOR] Texte en anglais détecté, traduction en français...");
            match translate(text, "en", "fr") {
                Ok(translated) => {
                    eprintln!("[TRANSLATOR] Traduction réussie");
                    Ok(translated)
                }
                Err(e) => {
                    eprintln!("[TRANSLATOR] Erreur de traduction: {}", e);
                    eprintln!("[TRANSLATOR] Utilisation du texte original en anglais");
                    Ok(text.to_string())
                }
            }
        }
        DetectedLanguage::French => Ok(text.to_string()),
    }
}

#[tauri::command]
fn register_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    eprintln!("[REGISTER] Début de register_global_shortcut()");
    let config = load_config()?;
    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);

    let shortcut = shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci: {:?}", e))?;

    eprintln!("[REGISTER] Raccourci à enregistrer: {}", shortcut_str);
    eprintln!("[REGISTER] Tentative de désenregistrement du raccourci précédent");

    // S'assurer que le raccourci précédent est désenregistré
    let unregister_result = app_handle.global_shortcut().unregister(shortcut);
    eprintln!(
        "[REGISTER] Résultat du désenregistrement: {:?}",
        unregister_result
    );

    eprintln!("[REGISTER] Enregistrement du raccourci");
    app_handle
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _accelerator, _state| {
            eprintln!("[REGISTER CALLBACK] Raccourci global déclenché!");
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("[REGISTER CALLBACK] Émission de global_shortcut_triggered");
                let _ = window.emit("global_shortcut_triggered", ());
            }
        })
        .map_err(|e| {
            format!(
                "Erreur lors de l'enregistrement du raccourci {}: {:?}",
                shortcut_str, e
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

    let text_to_speak = translate_to_french_if_english(&cleaned_text, detected_lang)?;

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
    tts.set_lang("fr".to_string());

    // Charger la vitesse sauvegardée
    let playback_speed = load_config().map(|cfg| cfg.playback_speed).unwrap_or(1.0);

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

    let text_to_speak = translate_to_french_if_english(&cleaned_text, detected_lang)?;

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
    tts.set_lang("fr".to_string());

    let playback_speed = load_config().map(|cfg| cfg.playback_speed).unwrap_or(1.0);

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
            register_global_shortcut,
            unregister_global_shortcut,
            load_playback_speed,
            save_playback_speed,
            load_dev_mode,
            save_dev_mode,
            speak,
            stop_speak,
            speak_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
