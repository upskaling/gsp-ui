//! Moteur de traduction basé sur LinguaSpark
//!
//! Utilise LinguaSpark pour la traduction multilingue sans dépendance externe.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use isolang::Language;
use linguaspark::{DecodeOptions, Executor, Model, ModelAssets, VocabularyAssets};
use log::{debug, info};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Moteur de traduction LinguaSpark
pub struct TranslationEngine {
    models: Arc<Mutex<HashMap<(Language, Language), Model>>>,
    executor: Arc<Mutex<Executor>>,
}

impl TranslationEngine {
    /// Crée un nouveau moteur en chargeant les modèles depuis un répertoire
    pub fn load(models_dir: &Path) -> Result<Self> {
        info!(
            "[TRANSLATION_ENGINE] Chargement des modèles depuis {}",
            models_dir.display()
        );

        let mut directories = fs::read_dir(models_dir)?.collect::<Result<Vec<_>, _>>()?;
        directories.sort_by_key(|entry| entry.file_name());

        let mut models = HashMap::new();

        for entry in directories {
            if !entry.path().is_dir() {
                continue;
            }

            let path = entry.path();
            let dir_name = entry.file_name().to_string_lossy().into_owned();

            // Parse language pair from directory name (e.g., "en-fr" or "enfr")
            let (source, target) = parse_language_pair(&dir_name)?;
            let source_code = iso_code(&source)?;
            let target_code = iso_code(&target)?;

            if models.contains_key(&(source, target)) {
                return Err(anyhow::anyhow!(
                    "Modèle dupliqué pour la paire linguistique '{}-{}'",
                    source_code,
                    target_code
                ));
            }

            info!(
                "[TRANSLATION_ENGINE] Chargement du modèle '{}-{}' depuis {}",
                source_code,
                target_code,
                path.display()
            );

            let assets = discover_model_assets(&path)?;
            let model = Model::from_assets(assets).context(format!(
                "Impossible de charger le modèle '{}-{}' depuis '{}'",
                source_code,
                target_code,
                path.display()
            ))?;

            models.insert((source, target), model);
        }

        if models.is_empty() {
            return Err(anyhow::anyhow!(
                "Aucun répertoire de modèles trouvé dans '{}'",
                models_dir.display()
            ));
        }

        info!(
            "[TRANSLATION_ENGINE] {} modèles chargés avec succès",
            models.len()
        );

        let executor = Executor::new().context("Impossible de créer l'exécuteur d'inférence")?;

        Ok(Self {
            models: Arc::new(Mutex::new(models)),
            executor: Arc::new(Mutex::new(executor)),
        })
    }

    /// Traduit un texte d'une langue source vers une langue cible
    pub fn translate(&self, text: &str, from: &str, to: &str) -> Result<String> {
        debug!(
            "[TRANSLATION_ENGINE] Traduction: {} -> {} | '{}'",
            from, to, text
        );

        let source_lang = parse_language_code(from)
            .context(format!("Code de langue source invalide: '{}'", from))?;
        let target_lang =
            parse_language_code(to).context(format!("Code de langue cible invalide: '{}'", to))?;

        // Rechercher le modèle direct en scope limité
        let model_found = {
            let models = self.models.lock().unwrap();
            models.contains_key(&(source_lang, target_lang))
        };

        if model_found {
            let models = self.models.lock().unwrap();
            let model = models.get(&(source_lang, target_lang)).unwrap();
            let mut executor = self.executor.lock().unwrap();
            return translate_with_model(&mut executor, model, text);
        }

        // Cherche une traduction via l'anglais (pivot)
        let eng = Language::Eng;
        let (has_to_eng, has_eng_to_target) = {
            let models = self.models.lock().unwrap();
            (
                models.contains_key(&(source_lang, eng)),
                models.contains_key(&(eng, target_lang)),
            )
        };

        if has_to_eng && has_eng_to_target {
            let models = self.models.lock().unwrap();
            let to_eng = models.get(&(source_lang, eng)).unwrap();
            let eng_to_target = models.get(&(eng, target_lang)).unwrap();
            let mut executor = self.executor.lock().unwrap();

            let intermediate = translate_with_model(&mut executor, to_eng, text)?;
            return translate_with_model(&mut executor, eng_to_target, &intermediate);
        }

        Err(anyhow::anyhow!(
            "Traduction de '{}' à '{}' n'est pas supportée",
            from,
            to
        ))
    }
}

fn translate_with_model(executor: &mut Executor, model: &Model, text: &str) -> Result<String> {
    let translations = executor
        .translate_batch(model, &[text.to_string()], &DecodeOptions::default())
        .context("Erreur lors de la traduction avec le modèle")?;

    translations
        .into_iter()
        .next()
        .map(|t| t.text)
        .ok_or_else(|| anyhow::anyhow!("Aucune traduction retournée"))
}

fn parse_language_code(code: &str) -> Result<Language> {
    let normalized = code.split('-').next().unwrap_or(code).to_ascii_lowercase();
    Language::from_639_1(&normalized).ok_or_else(|| {
        anyhow::anyhow!(
            "Code de langue invalide: '{}'. Format attendu: ISO 639-1",
            code
        )
    })
}

fn parse_language_pair(name: &str) -> Result<(Language, Language)> {
    let (source, target) = if name.len() == 4 && name.is_ascii() {
        (&name[..2], &name[2..])
    } else {
        let mut parts = name.split('-');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(source), Some(target), None) => (source, target),
            _ => {
                return Err(anyhow::anyhow!(
                    "Répertoire de modèles invalide '{}'; format attendu: 'en-fr' ou 'enfr'",
                    name
                ));
            }
        }
    };

    Ok((parse_language_code(source)?, parse_language_code(target)?))
}

fn iso_code(language: &Language) -> Result<&'static str> {
    if language.to_639_3() == "cmn" {
        return Ok("zh");
    }
    language
        .to_639_1()
        .ok_or_else(|| anyhow::anyhow!("Langue '{}' n'a pas de code ISO 639-1", language))
}

fn discover_model_assets(model_dir: &Path) -> Result<ModelAssets> {
    let mut model_path = None;
    let mut shortlist_path = None;
    let mut shared_vocab_path = None;
    let mut source_vocab_path = None;
    let mut target_vocab_path = None;

    for entry in fs::read_dir(model_dir)? {
        let entry = entry?;
        if !entry.path().is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let uncompressed_name = file_name.strip_suffix(".gz").unwrap_or(&file_name);

        if uncompressed_name.ends_with(".s2t.bin") {
            set_unique(&mut shortlist_path, path, "shortlist", model_dir)?;
        } else if uncompressed_name.ends_with(".bin") && uncompressed_name.contains(".intgemm") {
            set_unique(&mut model_path, path, "model", model_dir)?;
        } else if uncompressed_name.ends_with(".spm") {
            if uncompressed_name.starts_with("srcvocab") {
                set_unique(&mut source_vocab_path, path, "source vocabulary", model_dir)?;
            } else if uncompressed_name.starts_with("trgvocab") {
                set_unique(&mut target_vocab_path, path, "target vocabulary", model_dir)?;
            } else if uncompressed_name.starts_with("vocab") {
                set_unique(&mut shared_vocab_path, path, "shared vocabulary", model_dir)?;
            }
        }
    }

    let required = |path: Option<PathBuf>, kind: &str| {
        path.ok_or_else(|| {
            anyhow::anyhow!(
                "Fichier '{}' manquant dans le répertoire de modèles '{}'",
                kind,
                model_dir.display()
            )
        })
    };

    let model = read_asset(&required(model_path, "model")?)?;
    let shortlist = read_asset(&required(shortlist_path, "shortlist")?)?;

    let vocabularies = match (shared_vocab_path, source_vocab_path, target_vocab_path) {
        (Some(shared), None, None) => VocabularyAssets::Shared(read_asset(&shared)?),
        (None, Some(source), Some(target)) => VocabularyAssets::Separate {
            source: read_asset(&source)?,
            target: read_asset(&target)?,
        },
        _ => {
            return Err(anyhow::anyhow!(
                "Répertoire de modèles '{}' doit contenir soit un seul vocabulaire partagé, soit un vocabulaire source et cible",
                model_dir.display()
            ));
        }
    };

    Ok(ModelAssets {
        model,
        vocabularies,
        shortlist,
    })
}

fn read_asset(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    if path.extension().is_some_and(|ext| ext == "gz") {
        let mut decoded = Vec::new();
        GzDecoder::new(bytes.as_slice()).read_to_end(&mut decoded)?;
        Ok(decoded)
    } else {
        Ok(bytes)
    }
}

fn set_unique(
    slot: &mut Option<PathBuf>,
    path: PathBuf,
    kind: &str,
    model_dir: &Path,
) -> Result<()> {
    if slot.replace(path).is_some() {
        return Err(anyhow::anyhow!(
            "Fichiers '{}' multiples trouvés dans le répertoire de modèles '{}'",
            kind,
            model_dir.display()
        ));
    }
    Ok(())
}
