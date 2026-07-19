//! Détection et gestion des langues pour la synthèse vocale

use lingua::Language::{English, French};
use lingua::LanguageDetectorBuilder;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectedLanguage {
    French,
    English,
}

impl DetectedLanguage {}

pub fn detect_language(text: &str) -> DetectedLanguage {
    let languages = vec![English, French];
    let detector = LanguageDetectorBuilder::from_languages(&languages).build();

    match detector.detect_language_of(text) {
        Some(English) => DetectedLanguage::English,
        Some(French) | None => DetectedLanguage::French,
    }
}
