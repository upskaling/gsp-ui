mod tts;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tts::{TtsEngine, EspeakNg};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    #[serde(default)]
    clipboard_shortcut: ClipboardShortcut,
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
    let app_handle = app.handle().clone();

    if let Ok(config) = load_config() {
        let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);

        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                if let Err(e) = app_handle.global_shortcut().on_shortcut(shortcut.clone(), move |app, _accelerator, _state| {
                    eprintln!("Raccourci global déclenché!");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("global_shortcut_triggered", ());
                    }
                }) {
                    eprintln!("Erreur lors de l'enregistrement du raccourci: {:?}", e);
                    return Err(format!("Erreur lors de l'enregistrement du raccourci: {:?}", e));
                }
                eprintln!("Raccourci global enregistré avec succès: {}", shortcut_str);
                Ok(())
            }
            Err(e) => {
                eprintln!("Erreur lors du parsing du raccourci {}: {:?}", shortcut_str, e);
                Err(format!("Erreur lors du parsing du raccourci: {:?}", e))
            }
        }
    } else {
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
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_clipboard_content() -> Result<String, String> {
    use x11_clipboard::Clipboard;

    let clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let atoms = clipboard.getter.atoms.clone();
    let timeout = std::time::Duration::from_secs(1);

    clipboard
        .load(clipboard.setter.atoms.primary, atoms.utf8_string, atoms.property, timeout)
        .map_err(|e| format!("Erreur lors de la lecture du presse-papier: {}", e))
        .and_then(|data| {
            String::from_utf8(data).map_err(|e| format!("Erreur de décodage UTF-8: {}", e))
        })
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

#[tauri::command]
fn register_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    let config = load_config()?;
    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);

    let shortcut = shortcut_str.parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci: {:?}", e))?;

    eprintln!("Enregistrement du raccourci global: {}", shortcut_str);

    // S'assurer que le raccourci précédent est désenregistré
    let _ = app_handle.global_shortcut().unregister(shortcut.clone());

    app_handle
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _accelerator, _state| {
            eprintln!("Raccourci global déclenché!");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("global_shortcut_triggered", ());
            }
        })
        .map_err(|e| format!("Erreur lors de l'enregistrement du raccourci {}: {:?}", shortcut_str, e))
}

#[tauri::command]
fn unregister_global_shortcut(app_handle: AppHandle) -> Result<(), String> {
    let config = load_config()?;
    let shortcut_str = shortcut_to_string(&config.clipboard_shortcut);

    let shortcut = shortcut_str.parse::<Shortcut>()
        .map_err(|e| format!("Erreur lors du parsing du raccourci: {:?}", e))?;

    app_handle
        .global_shortcut()
        .unregister(shortcut)
        .map_err(|e| format!("Erreur lors du désenregistrement du raccourci: {}", e))
}

#[tauri::command]
fn speak(text: String) -> Result<(), String> {
    let tts = EspeakNg::new();
    tts.speak(&text);
    Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            greet,
            get_clipboard_content,
            load_shortcut_config,
            save_shortcut_config,
            register_global_shortcut,
            unregister_global_shortcut,
            speak
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
