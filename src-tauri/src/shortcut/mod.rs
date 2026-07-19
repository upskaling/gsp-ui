//! Gestion modulée des raccourcis clavier avec support de plusieurs implémentations

pub mod handler;
pub mod handy_keys;
pub mod settings;
mod tauri_impl;

pub use settings::{ShortcutBinding, ShortcutConfig};

use log::{error, info};
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;

/// Initialiser tous les raccourcis depuis la configuration persistante
pub fn init_shortcuts(app: &AppHandle) {
    info!("Initialisation des raccourcis clavier avec tauri-plugin-global-shortcut");
    // Utiliser tauri-plugin-global-shortcut qui supporte super/meta
    tauri_impl::init_shortcuts(app);
}

/// Enregistrer un raccourci
pub fn register_shortcut(app: &AppHandle, binding_id: &str, shortcut_str: &str) -> Result<(), String> {
    // Utiliser l'implémentation tauri
    tauri_impl::register_shortcut(app, binding_id, shortcut_str)
}

/// Désenregistrer un raccourci
pub fn unregister_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    // Utiliser l'implémentation tauri
    tauri_impl::unregister_shortcut(app, shortcut_str)
}

// ============================================================================
// Commandes Tauri pour la gestion des raccourcis
// ============================================================================

#[derive(Serialize, Type)]
pub struct BindingResponse {
    pub success: bool,
    pub binding: Option<ShortcutBinding>,
    pub error: Option<String>,
}

/// Charger un raccourci spécifique
#[tauri::command]
pub fn get_binding(id: String) -> Result<ShortcutBinding, String> {
    let config = settings::load_shortcut_config()?;
    config
        .bindings
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("Raccourci '{}' non trouvé", id))
}

/// Charger tous les raccourcis
#[tauri::command]
pub fn get_all_bindings() -> Result<Vec<ShortcutBinding>, String> {
    let config = settings::load_shortcut_config()?;
    Ok(config.bindings.into_values().collect())
}

/// Changer un raccourci
#[tauri::command]
pub fn change_binding(app: AppHandle, id: String, binding: String) -> Result<BindingResponse, String> {
    if binding.trim().is_empty() {
        return Err("Le raccourci ne peut pas être vide".to_string());
    }

    let mut config = settings::load_shortcut_config()?;

    let existing_binding = config.bindings.get(&id).cloned().ok_or_else(|| {
        format!("Raccourci '{}' non trouvé", id)
    })?;

    // Désenregistrer l'ancien raccourci
    if let Err(e) = unregister_shortcut(&app, &existing_binding.current_binding) {
        error!("Erreur de désenregistrement de l'ancien raccourci: {}", e);
    }

    // Valider le nouveau raccourci avec tauri
    if let Err(e) = tauri_impl::validate_shortcut(&binding) {
        return Err(e);
    }

    // Enregistrer le nouveau raccourci
    if let Err(e) = register_shortcut(&app, &id, &binding) {
        error!("Erreur d'enregistrement du nouveau raccourci: {}", e);
        return Ok(BindingResponse {
            success: false,
            binding: None,
            error: Some(e),
        });
    }

    // Mettre à jour la configuration
    let mut updated_binding = existing_binding;
    updated_binding.current_binding = binding;
    config.bindings.insert(id, updated_binding.clone());
    settings::save_shortcut_config(&config)?;

    Ok(BindingResponse {
        success: true,
        binding: Some(updated_binding),
        error: None,
    })
}

/// Réinitialiser un raccourci à sa valeur par défaut
#[tauri::command]
pub fn reset_binding(app: AppHandle, id: String) -> Result<BindingResponse, String> {
    let config = settings::load_shortcut_config()?;

    let binding = config.bindings.get(&id).cloned().ok_or_else(|| {
        format!("Raccourci '{}' non trouvé", id)
    })?;

    change_binding(app, id, binding.default_binding)
}

/// Suspendre un raccourci (désenregistrer temporairement)
#[tauri::command]
pub fn suspend_binding(app: AppHandle, id: String) -> Result<(), String> {
    let config = settings::load_shortcut_config()?;

    if let Some(binding) = config.bindings.get(&id) {
        unregister_shortcut(&app, &binding.current_binding)?;
    }

    Ok(())
}

/// Reprendre un raccourci (réenregistrer)
#[tauri::command]
pub fn resume_binding(app: AppHandle, id: String) -> Result<(), String> {
    let config = settings::load_shortcut_config()?;

    if let Some(binding) = config.bindings.get(&id) {
        register_shortcut(&app, &id, &binding.current_binding)?;
    }

    Ok(())
}

/// Formater un raccourci dans le format correct pour tauri-plugin-global-shortcut
#[tauri::command]
pub fn format_shortcut_binding(
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
    key: String,
) -> Result<String, String> {
    let mut parts: Vec<String> = Vec::new();

    if ctrl {
        parts.push("ctrl".to_string());
    }
    if shift {
        parts.push("shift".to_string());
    }
    if alt {
        parts.push("alt".to_string());
    }
    if meta {
        // tauri-plugin-global-shortcut supporte 'super' comme modificateur
        parts.push("super".to_string());
    }

    let key_lower = key.to_lowercase();
    parts.push(key_lower);

    Ok(parts.join("+"))
}
