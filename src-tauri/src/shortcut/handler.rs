//! Gestionnaire d'événements de raccourcis partagé

use log::warn;
use tauri::{AppHandle, Emitter, Manager};

/// Gérer un événement de raccourci depuis l'implémentation Tauri
pub fn handle_shortcut_event(app: &AppHandle, binding_id: &str, _shortcut_string: &str) {
    match binding_id {
        "clipboard" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("global_shortcut_triggered", ());
            }
        }
        "ocr" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("global_shortcut_ocr_triggered", ());
            }
        }
        other => {
            warn!("Raccourci inconnu: {}", other);
        }
    }
}
