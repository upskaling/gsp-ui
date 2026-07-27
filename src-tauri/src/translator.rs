//! Traduction locale avec LinguaSpark
//!
//! Fournit une traduction locale en utilisant le moteur LinguaSpark (multilingue).
//! Les modèles sont téléchargés automatiquement si manquants.

use crate::translation_engine::TranslationEngine;
use crate::model_downloader;
use log::info;
use std::sync::OnceLock;

static TRANSLATION_ENGINE: OnceLock<TranslationEngine> = OnceLock::new();

/// Initialise le moteur de traduction
/// Doit être appelé avant toute traduction
///
/// # Comportement
/// - Crée le répertoire des modèles s'il n'existe pas
/// - Télécharge les modèles par défaut (en-fr, fr-en) si absent
/// - Initialise le moteur LinguaSpark
pub fn initialize_engine() -> Result<(), String> {
    if TRANSLATION_ENGINE.get().is_some() {
        return Ok(());
    }

    // Initialiser les modèles (créer répertoire + télécharger si besoin)
    let models_dir = model_downloader::initialize_models()
        .map_err(|e| format!("Erreur lors de l'initialisation des modèles: {}", e))?;

    info!("[TRANSLATOR] Modèles stockés dans: {}", models_dir.display());

    if !models_dir.exists() {
        return Err(format!(
            "Répertoire des modèles n'existe pas: {}",
            models_dir.display()
        ));
    }

    // Essayer de charger les modèles disponibles
    match TranslationEngine::load(&models_dir) {
        Ok(engine) => {
            TRANSLATION_ENGINE.get_or_init(|| engine);
            info!("[TRANSLATOR] Moteur de traduction initialisé avec succès");
            Ok(())
        }
        Err(e) => {
            // L'app fonctionne sans traduction, ce n'est pas critique
            Err(format!(
                "Moteur de traduction non disponible: {}. L'app fonctionnera sans traduction.",
                e
            ))
        }
    }
}

/// Traduit un texte d'une langue source vers une langue cible
///
/// # Arguments
/// * `text` - Le texte à traduire
/// * `lang_from` - Code de langue source (ex: "en")
/// * `lang_to` - Code de langue cible (ex: "fr")
///
/// # Retour
/// Retourne le texte traduit ou une erreur
pub fn translate(text: &str, lang_from: &str, lang_to: &str) -> Result<String, String> {
    let engine = TRANSLATION_ENGINE.get()
        .ok_or_else(|| "Moteur de traduction non initialisé".to_string())?;

    engine
        .translate(text, lang_from, lang_to)
        .map_err(|e| format!("Erreur de traduction: {}", e))
}
