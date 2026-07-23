use log::{error, warn};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use super::handler::handle_shortcut_event;

/// Initialiser tous les raccourcis depuis la configuration
pub fn init_shortcuts(app: &AppHandle) {
    use super::settings::load_shortcut_config;

    match load_shortcut_config() {
        Ok(config) => {
            for (id, binding) in config.bindings {
                if let Err(e) = register_shortcut(app, &id, &binding.current_binding) {
                    error!(
                        "Erreur d'enregistrement du raccourci '{}' au démarrage: {}",
                        id, e
                    );
                }
            }
        }
        Err(e) => {
            error!("Erreur de chargement de la configuration: {}", e);
        }
    }
}

/// Valider un raccourci pour Tauri
pub fn validate_shortcut(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Le raccourci ne peut pas être vide".into());
    }

    let modifiers = ["ctrl", "shift", "alt", "meta", "super"];

    let parts: Vec<String> = raw.split('+').map(|p| p.trim().to_lowercase()).collect();

    let has_non_modifier = parts.iter().any(|part| !modifiers.contains(&part.as_str()));

    if has_non_modifier {
        Ok(())
    } else {
        Err("Le raccourci doit inclure une touche principale (lettre, nombre, F-key, etc.) en plus des modificateurs"
            .into())
    }
}

/// Enregistrer un raccourci avec un ID et une chaîne de raccourci
pub fn register_shortcut(
    app: &AppHandle,
    binding_id: &str,
    shortcut_str: &str,
) -> Result<(), String> {
    if let Err(e) = validate_shortcut(shortcut_str) {
        warn!(
            "Erreur de validation du raccourci '{}' pour '{}': {}",
            shortcut_str, binding_id, e
        );
        return Err(e);
    }

    let shortcut = match shortcut_str.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("Erreur d'analyse du raccourci '{}': {}", shortcut_str, e);
            error!("Erreur d'analyse: {}", error_msg);
            return Err(error_msg);
        }
    };

    if app.global_shortcut().is_registered(shortcut) {
        let error_msg = format!("Le raccourci '{}' est déjà utilisé", shortcut_str);
        warn!("Erreur de duplicate: {}", error_msg);
        return Err(error_msg);
    }

    let binding_id_for_closure = binding_id.to_string();
    let shortcut_str_for_closure = shortcut_str.to_string();
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _accelerator, _state| {
            handle_shortcut_event(app, &binding_id_for_closure, &shortcut_str_for_closure);
        })
        .map_err(|e| {
            let error_msg = format!(
                "Impossible d'enregistrer le raccourci '{}': {}",
                shortcut_str, e
            );
            error!("Erreur d'enregistrement: {}", error_msg);
            error_msg
        })?;

    Ok(())
}

/// Désenregistrer un raccourci
pub fn unregister_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut = match shortcut_str.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Erreur d'analyse du raccourci '{}' pour désenregistrement: {}",
                shortcut_str, e
            );
            error!("Erreur d'analyse: {}", error_msg);
            return Err(error_msg);
        }
    };

    app.global_shortcut().unregister(shortcut).map_err(|e| {
        let error_msg = format!(
            "Erreur de désenregistrement du raccourci '{}': {}",
            shortcut_str, e
        );
        error!("Erreur de désenregistrement: {}", error_msg);
        error_msg
    })?;

    Ok(())
}
