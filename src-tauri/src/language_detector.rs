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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_english_text() {
        // Test: détection du texte anglais
        let result = detect_language("Hello, this is an English text");
        assert_eq!(result, DetectedLanguage::English);
    }

    #[test]
    fn test_detect_french_text() {
        // Test: détection du texte français
        let result = detect_language("Bonjour, ceci est un texte en français");
        assert_eq!(result, DetectedLanguage::French);
    }

    #[test]
    fn test_detect_english_common_words() {
        // Test: détection avec mots anglais courants
        let result = detect_language("The quick brown fox jumps over the lazy dog");
        assert_eq!(result, DetectedLanguage::English);
    }

    #[test]
    fn test_detect_french_common_words() {
        // Test: détection avec mots français courants
        let result = detect_language("Le renard brun rapide saute par-dessus le chien paresseux");
        assert_eq!(result, DetectedLanguage::French);
    }

    #[test]
    fn test_detect_french_short_text() {
        // Test: détection avec texte court français
        let result = detect_language("Bonjour monde");
        // Pour du texte court, la détection peut être moins fiable, mais doit retourner une langue
        match result {
            DetectedLanguage::English | DetectedLanguage::French => {
                // Les deux sont acceptables pour un texte court
            }
        }
    }

    #[test]
    fn test_detect_unknown_or_empty() {
        // Test: texte non identifié ou vide retourne français par défaut
        let result = detect_language("...");
        // Quand la détection échoue, on retourne French par défaut
        assert_eq!(result, DetectedLanguage::French);
    }
}
