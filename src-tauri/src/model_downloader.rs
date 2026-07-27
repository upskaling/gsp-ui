//! Téléchargement automatique des modèles de traduction Bergamot
//!
//! Télécharge les modèles Bergamot-Translator depuis data.statmt.org
//! et les stocke localement pour utilisation par LinguaSpark.
//!
//! Les modèles Bergamot contiennent les fichiers nécessaires pour LinguaSpark:
//! - model.intgemm.*.bin (fichier modèle)
//! - lex.s2t.bin ou *.s2t.bin (shortlist)
//! - vocab.*.spm (vocabulaire SentencePiece)
//!
//! Source: https://data.statmt.org/bergamot/models/

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;

/// Modèles Bergamot-Translator disponibles sur data.statmt.org
/// Format: (lang_from, lang_to, direct_url)
const DEFAULT_MODELS: &[(&str, &str, &str)] = &[
    // English-French tiny model (from mozilla bergamot repository)
    ("en", "fr", "https://data.statmt.org/bergamot/models/fren/enfr.student.tiny11.v1.805d112122af03d0.tar.gz"),
    // French-English tiny model (from mozilla bergamot repository)
    ("fr", "en", "https://data.statmt.org/bergamot/models/fren/fren.student.tiny11.v1.dccea16d03c0a389.tar.gz"),
];

/// Obtient le répertoire de stockage des modèles
/// Utilise toujours le répertoire config utilisateur pour éviter les boucles
/// infinies lors du développement (file watcher qui détecte les changements de modèles)
pub fn get_models_dir() -> PathBuf {
    if let Ok(config_dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config_dir).join("gsp-ui").join("models")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config").join("gsp-ui").join("models")
    } else {
        PathBuf::from("/tmp/gsp-ui/models")
    }
}

/// Initialise les modèles (crée le répertoire s'il n'existe pas)
pub fn initialize_models() -> Result<PathBuf> {
    let models_dir = get_models_dir();

    if !models_dir.exists() {
        info!("[MODEL_DOWNLOADER] Création du répertoire de modèles: {}", models_dir.display());
        fs::create_dir_all(&models_dir)
            .context(format!(
                "Impossible de créer le répertoire des modèles: {}",
                models_dir.display()
            ))?;
    }

    // Vérifier si des modèles sont déjà présents
    match check_existing_models(&models_dir) {
        Ok(found) => {
            if found > 0 {
                info!("[MODEL_DOWNLOADER] {} modèle(s) existant(s) trouvé(s)", found);
                return Ok(models_dir);
            }
        }
        Err(e) => {
            warn!("[MODEL_DOWNLOADER] Erreur lors de la vérification des modèles: {}", e);
        }
    }

    // Essayer de télécharger les modèles par défaut
    if let Err(e) = download_default_models(&models_dir) {
        warn!(
            "[MODEL_DOWNLOADER] Impossible de télécharger les modèles: {}. L'app fonctionnera sans traduction.",
            e
        );
    }

    Ok(models_dir)
}

/// Vérifie combien de modèles existent déjà
fn check_existing_models(models_dir: &PathBuf) -> Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(models_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Vérifier si c'est un répertoire de modèle valide
            if is_valid_model_dir(&path) {
                count += 1;
                let dir_name = entry.file_name();
                info!(
                    "[MODEL_DOWNLOADER] Modèle valide trouvé: {}",
                    dir_name.to_string_lossy()
                );
            }
        }
    }
    Ok(count)
}

/// Vérifie si un répertoire contient un modèle valide
fn is_valid_model_dir(path: &PathBuf) -> bool {
    // Un modèle valide doit contenir:
    // - *.intgemm.*.bin (modèle)
    // - *.s2t.bin (shortlist)
    // - *.spm ou *.spm.gz (vocabulaire)

    if !path.is_dir() {
        return false;
    }

    if let Ok(entries) = fs::read_dir(path) {
        let file_names: Vec<String> = entries
            .filter_map(|e| e.ok().and_then(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                // Ignorer les fichiers de sauvegarde ou temporaires
                if name.ends_with(".backup") || name.starts_with(".") {
                    None
                } else {
                    Some(name)
                }
            }))
            .collect();

        let has_model = file_names.iter().any(|f| {
            f.contains(".intgemm.") && f.ends_with(".bin")
        });
        let has_shortlist = file_names.iter().any(|f| f.ends_with(".s2t.bin"));
        let has_vocab = file_names.iter().any(|f| {
            f.ends_with(".spm") || f.ends_with(".spm.gz")
        });

        return has_model && has_shortlist && has_vocab;
    }

    false
}

/// Télécharge les modèles par défaut (en-fr, fr-en) via HTTP
fn download_default_models(models_dir: &PathBuf) -> Result<()> {
    info!("[MODEL_DOWNLOADER] Tentative de téléchargement des modèles par défaut...");

    for (from, to, model_url) in DEFAULT_MODELS {
        let pair_dir = format!("{}{}", from, to);
        let target_path = models_dir.join(&pair_dir);

        if is_valid_model_dir(&target_path) {
            debug!(
                "[MODEL_DOWNLOADER] Modèle '{}' déjà existant, skip",
                pair_dir
            );
            continue;
        }

        info!("[MODEL_DOWNLOADER] Téléchargement du modèle '{}'...", pair_dir);

        if let Err(e) = try_download_model_from_url(model_url, &target_path) {
            warn!(
                "[MODEL_DOWNLOADER] Impossible de télécharger '{}': {}",
                pair_dir, e
            );
        }
    }

    Ok(())
}

/// Télécharge un modèle depuis une URL Bergamot spécifique
fn try_download_model_from_url(model_url: &str, target_path: &PathBuf) -> Result<()> {
    // Créer le répertoire cible
    fs::create_dir_all(target_path)
        .context(format!(
            "Impossible de créer le répertoire: {}",
            target_path.display()
        ))?;

    // Extraire le nom du fichier depuis l'URL
    let archive_filename = model_url
        .split('/')
        .last()
        .ok_or_else(|| anyhow::anyhow!("URL invalide: {}", model_url))?;

    let temp_file = std::env::temp_dir()
        .join(format!("gsp-ui-model-{}-{}", uuid::Uuid::new_v4(), archive_filename));

    debug!("[MODEL_DOWNLOADER] Téléchargement depuis: {}", model_url);

    // Télécharger le fichier via HTTP avec timeout
    let response = ureq::get(model_url)
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes
        .call()
        .context(format!("Erreur lors du téléchargement de {}", model_url))?;

    let mut temp_file_handle = fs::File::create(&temp_file)
        .context(format!("Impossible de créer le fichier temporaire: {}", temp_file.display()))?;

    std::io::copy(&mut response.into_reader(), &mut temp_file_handle)
        .context("Erreur lors de l'écriture du fichier téléchargé")?;

    info!("[MODEL_DOWNLOADER] Archive téléchargée: {}", archive_filename);
    extract_tar_gz(&temp_file, target_path)?;

    if let Err(e) = fs::remove_file(&temp_file) {
        warn!(
            "[MODEL_DOWNLOADER] Impossible de supprimer le fichier temporaire: {}",
            e
        );
    }

    info!("[MODEL_DOWNLOADER] Modèle extracté avec succès");
    Ok(())
}

/// Extrait une archive tar.gz et gère les sous-répertoires imbriqués
fn extract_tar_gz(archive_path: &PathBuf, target_dir: &PathBuf) -> Result<()> {
    // Extraire dans un répertoire temporaire d'abord
    let temp_extract_dir = std::env::temp_dir().join(format!(
        "gsp-ui-extract-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&temp_extract_dir)?;

    // Extraire avec flate2 (Rust natif, pas de dépendance externe)
    let tar_gz = fs::File::open(archive_path)
        .context(format!("Impossible d'ouvrir l'archive: {}", archive_path.display()))?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    archive
        .unpack(&temp_extract_dir)
        .context("Erreur lors de l'extraction de l'archive tar.gz")?;

    // Chercher le répertoire contenant les fichiers de modèle
    // (gérer les cas où les fichiers sont dans un sous-répertoire)
    let extracted_files = fs::read_dir(&temp_extract_dir)?
        .collect::<Result<Vec<_>, _>>()?;

    let model_dir = if extracted_files.len() == 1 && extracted_files[0].path().is_dir() {
        // Si un seul répertoire, c'est probablement le répertoire du modèle
        extracted_files[0].path()
    } else {
        // Sinon, les fichiers sont directement à la racine
        temp_extract_dir.clone()
    };

    // Copier les fichiers de modèle vers le répertoire cible
    for entry in fs::read_dir(&model_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_name = entry.file_name();
            let dest_path = target_dir.join(&file_name);
            fs::copy(&path, &dest_path)
                .context(format!("Impossible de copier {}", file_name.to_string_lossy()))?;
        }
    }

    // Nettoyer le répertoire temporaire
    if let Err(e) = fs::remove_dir_all(&temp_extract_dir) {
        warn!(
            "[MODEL_DOWNLOADER] Impossible de supprimer le répertoire temporaire: {}",
            e
        );
    }

    Ok(())
}

