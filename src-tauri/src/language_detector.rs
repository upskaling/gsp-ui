//! Détection et gestion des langues pour la synthèse vocale

use lingua::Language::{English, French};
use lingua::LanguageDetectorBuilder;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectedLanguage {
    French,
    English,
}

impl DetectedLanguage {
    pub fn as_espeak_code(&self) -> &'static str {
        match self {
            DetectedLanguage::French => "fr",
            DetectedLanguage::English => "en",
        }
    }

    pub fn as_espeak_voice(&self) -> &'static str {
        match self {
            DetectedLanguage::French => "mb-FR4",
            DetectedLanguage::English => "mb-EN1",
        }
    }
}

pub fn detect_language(text: &str) -> DetectedLanguage {
    let languages = vec![English, French];
    let detector = LanguageDetectorBuilder::from_languages(&languages).build();

    match detector.detect_language_of(text) {
        Some(English) => DetectedLanguage::English,
        Some(French) | None => DetectedLanguage::French,
    }
}
