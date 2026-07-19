//! Moteur de synthèse vocale eSpeak-NG
//!
//! Implémentation du trait TtsEngine pour eSpeak-NG.

use std::process::{Command, Child};
use crate::textutils::{trim_whitespace, remove_special_characters, read_vars};

/// Trait pour les moteurs de synthèse vocale
pub trait TtsEngine {
    fn speak(&self, text: &str) -> Result<Child, String>;
    fn set_lang(&mut self, lang: String) -> &mut Self;
    fn set_speed(&mut self, speed: i32) -> &mut Self;
}

/// Configuration du moteur eSpeak-NG
#[derive(Debug, Clone)]
pub struct EspeakNg {
    lang: String,
    speed: i32,
    pitch: i32,
    amplitude: i32,
    output_file: String,
}

impl Default for EspeakNg {
    fn default() -> Self {
        Self {
            lang: "fr".to_string(),
            speed: 100,
            pitch: 50,
            amplitude: 100,
            output_file: "/dev/shm/out.wav".to_string(),
        }
    }
}

impl EspeakNg {
    /// Crée une nouvelle configuration eSpeak-NG avec des valeurs par défaut
    pub fn new() -> Self {
        Self::default()
    }
}

/// Prétraite le texte (nettoyage, formatage)
fn preprocess_text(text: String) -> String {
    let mut text = text;
    text = read_vars(&text);
    text = remove_special_characters(&text);
    text = trim_whitespace(&text);
    text
}

impl TtsEngine for EspeakNg {
    fn speak(&self, text: &str) -> Result<Child, String> {
        let preprocessed_text = preprocess_text(text.to_string());
        let speed = (self.speed as f32 / 100.0 * 320.0) as i32 / 2;

        let result = Command::new("espeak-ng")
            .arg("-v")
            .arg(format!("mb-{}{}", self.lang[..2].to_uppercase(), "4"))
            .arg("-s")
            .arg(speed.to_string())
            .arg("-p")
            .arg(self.pitch.to_string())
            .arg("-a")
            .arg(self.amplitude.to_string())
            .arg("-w")
            .arg(self.output_file.as_str())
            .arg("--")
            .arg(preprocessed_text)
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    return Err(format!("Erreur espeak-ng: {}", String::from_utf8_lossy(&output.stderr)));
                }

                eprintln!("Audio généré: {}", self.output_file);
                let child = Command::new("paplay")
                    .arg(self.output_file.as_str())
                    .spawn()
                    .map_err(|e| format!("Erreur lors du lancement de paplay: {}", e))?;

                Ok(child)
            }
            Err(e) => {
                Err(format!("Erreur lors de l'exécution d'eSpeak-NG: {}", e))
            }
        }
    }

    fn set_lang(&mut self, lang: String) -> &mut Self {
        self.lang = lang;
        self
    }

    fn set_speed(&mut self, speed: i32) -> &mut Self {
        self.speed = speed;
        self
    }
}
