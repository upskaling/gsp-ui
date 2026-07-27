//! Moteurs de synthèse vocale
//!
//! Utilise eSpeak-NG sur Linux et le moteur TTS intégré (say) sur macOS.

use log::debug;
use std::process::Command;

/// Trait pour les moteurs de synthèse vocale
pub trait TtsEngine {
    fn speak(&self, text: &str) -> Result<String, String>;
    fn set_lang(&mut self, lang: String);
    fn set_speed(&mut self, speed: i32);
}

/// Configuration du moteur eSpeak-NG (utilisé sur Linux)
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct EspeakNg {
    lang: String,
    speed: i32,
    pitch: i32,
    amplitude: i32,
    output_file: String,
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
impl EspeakNg {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(target_os = "linux")]
impl TtsEngine for EspeakNg {
    fn speak(&self, text: &str) -> Result<String, String> {
        let speed = (self.speed as f32 / 100.0 * 320.0) as i32 / 2;

        let voice = if self.lang == "en" {
            "mb-EN1"
        } else {
            "mb-FR4"
        };

        let result = Command::new("espeak-ng")
            .arg("-v")
            .arg(voice)
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
                    return Err(format!(
                        "Erreur espeak-ng: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                debug!("[TTS] Audio généré: {}", self.output_file);
                Ok(self.output_file.clone())
            }
            Err(e) => Err(format!("Erreur lors de l'exécution d'eSpeak-NG: {}", e)),
        }
    }

    fn set_lang(&mut self, lang: String) {
        self.lang = lang;
    }

    fn set_speed(&mut self, speed: i32) {
        self.speed = speed;
    }
}

/// Moteur de synthèse vocale macOS utilisant `say` et `afconvert`.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct MacOsTts {
    lang: String,
    speed: i32,
}

#[cfg(target_os = "macos")]
impl Default for MacOsTts {
    fn default() -> Self {
        Self {
            lang: "fr".to_string(),
            speed: 200,
        }
    }
}

#[cfg(target_os = "macos")]
impl MacOsTts {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_voice(&self) -> &str {
        match self.lang.as_str() {
            "en" => "Samantha",
            _ => "Thomas",
        }
    }
}

#[cfg(target_os = "macos")]
impl TtsEngine for MacOsTts {
    fn speak(&self, text: &str) -> Result<String, String> {
        let aiff_path = std::env::temp_dir().join("gsp-ui.aiff");
        let wav_path = std::env::temp_dir().join("gsp-ui.wav");
        let aiff_str = aiff_path.to_str().ok_or("Chemin AIFF invalide")?;
        let wav_str = wav_path.to_str().ok_or("Chemin WAV invalide")?;

        let output = Command::new("say")
            .arg("-v")
            .arg(self.get_voice())
            .arg("-r")
            .arg(self.speed.to_string())
            .arg("-o")
            .arg(aiff_str)
            .arg(text)
            .output()
            .map_err(|e| format!("Erreur say: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Erreur say: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let convert = Command::new("afconvert")
            .arg("-f")
            .arg("WAVE")
            .arg("-d")
            .arg("LEI16")
            .arg(aiff_str)
            .arg(wav_str)
            .output()
            .map_err(|e| format!("Erreur afconvert: {}", e))?;

        if !convert.status.success() {
            return Err(format!(
                "Erreur afconvert: {}",
                String::from_utf8_lossy(&convert.stderr)
            ));
        }

        if let Err(e) = std::fs::remove_file(&aiff_path) {
            debug!("[TTS] Nettoyage AIFF: {}", e);
        }

        debug!("[TTS] Audio généré: {}", wav_str);
        Ok(wav_str.to_string())
    }

    fn set_lang(&mut self, lang: String) {
        self.lang = lang;
    }

    fn set_speed(&mut self, speed: i32) {
        self.speed = speed;
    }
}

/// Crée une instance du moteur TTS appropriée à la plateforme
pub fn create_tts_engine() -> Box<dyn TtsEngine> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsTts::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(EspeakNg::new())
    }
}

/// Convertit la vitesse de lecture frontend en valeur TTS selon la plateforme
pub fn convert_playback_speed(playback_speed: f32) -> i32 {
    #[cfg(target_os = "macos")]
    {
        (playback_speed * 200.0) as i32
    }
    #[cfg(target_os = "linux")]
    {
        ((playback_speed * 100.0) as i32).clamp(50, 200)
    }
}
