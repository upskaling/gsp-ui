//! Gestionnaire d'événements de raccourcis partagé

use crate::{is_audio_playing, run_speak_clipboard, run_speak_ocr, run_stop_speak};
use log::{debug, error, warn};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Anti-rebond du traitement direct (l'interface gère le sien côté frontend).
static LAST_DIRECT_TRIGGER: Mutex<Option<Instant>> = Mutex::new(None);

fn direct_trigger_allowed() -> bool {
    let Ok(mut last) = LAST_DIRECT_TRIGGER.lock() else {
        return false;
    };

    let now = Instant::now();
    if let Some(previous) = *last {
        if now.duration_since(previous) < Duration::from_secs(1) {
            return false;
        }
    }

    *last = Some(now);
    true
}

/// Gérer un événement de raccourci depuis l'implémentation Tauri
pub fn handle_shortcut_event(app: &AppHandle, binding_id: &str, _shortcut_string: &str) {
    // L'interface graphique est disponible : c'est elle qui pilote la lecture.
    if let Some(window) = app.get_webview_window("main") {
        match binding_id {
            "clipboard" => {
                let _ = window.emit("global_shortcut_triggered", ());
            }
            "ocr" => {
                let _ = window.emit("global_shortcut_ocr_triggered", ());
            }
            other => {
                warn!("Raccourci inconnu: {}", other);
            }
        }
        return;
    }

    // L'interface a été libérée pour économiser la mémoire : on agit
    // directement depuis Rust pour que les raccourcis continuent de marcher.
    if !direct_trigger_allowed() {
        debug!("[SHORTCUT] Appel ignoré (anti-rebond)");
        return;
    }

    if is_audio_playing() {
        debug!("[SHORTCUT] Interface absente, arrêt de la lecture");
        if let Err(e) = run_stop_speak() {
            error!("[SHORTCUT] Erreur lors de l'arrêt de la lecture: {}", e);
        }
        return;
    }

    // La génération TTS est bloquante : on ne doit pas bloquer le thread qui
    // écoute les raccourcis.
    let app = app.clone();
    match binding_id {
        "clipboard" => {
            debug!("[SHORTCUT] Interface absente, lecture du presse-papier");
            std::thread::spawn(move || {
                if let Err(e) = run_speak_clipboard(app) {
                    error!(
                        "[SHORTCUT] Erreur lors de la lecture du presse-papier: {}",
                        e
                    );
                }
            });
        }
        "ocr" => {
            debug!("[SHORTCUT] Interface absente, lecture OCR");
            std::thread::spawn(move || {
                if let Err(e) = run_speak_ocr(app) {
                    error!("[SHORTCUT] Erreur lors de la lecture OCR: {}", e);
                }
            });
        }
        other => {
            warn!("Raccourci inconnu: {}", other);
        }
    }
}
