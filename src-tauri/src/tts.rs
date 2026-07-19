//! Moteur de synthèse vocale eSpeak-NG
//!
//! Implémentation du trait TtsEngine pour eSpeak-NG.

use std::process::Command;

/// Trait pour les moteurs de synthèse vocale
pub trait TtsEngine {
    fn speak(&self, text: &str);
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

impl TtsEngine for EspeakNg {
    fn speak(&self, text: &str) {
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
            .arg(text)
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("Erreur espeak-ng: {}", String::from_utf8_lossy(&output.stderr));
                } else {
                    eprintln!("Audio généré: {}", self.output_file);
                    // Jouer le fichier avec paplay
                    let _ = Command::new("paplay")
                        .arg(self.output_file.as_str())
                        .output();
                }
            }
            Err(e) => {
                eprintln!("Erreur lors de l'exécution d'eSpeak-NG: {}", e);
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
