//! Moteurs de synthèse vocale
//!
//! Utilise Piper TTS via CLI sur Linux et le moteur TTS intégré (say) sur macOS.

use log::debug;
use std::process::Command;

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

/// Trait pour les moteurs de synthèse vocale
pub trait TtsEngine {
    fn speak(&mut self, text: &str) -> Result<String, String>;
    fn set_lang(&mut self, lang: String);
    fn set_speed(&mut self, speed: i32);
}

/// Configuration du moteur Piper TTS via CLI (utilisé sur Linux)
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct PiperTts {
    lang: String,
    speed: f32,
    output_file: String,
}

#[cfg(target_os = "linux")]
impl Default for PiperTts {
    fn default() -> Self {
        Self {
            lang: "fr".to_string(),
            speed: 1.0,
            output_file: "/dev/shm/out.wav".to_string(),
        }
    }
}

#[cfg(target_os = "linux")]
impl PiperTts {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_piper_dir() -> Result<std::path::PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Impossible de trouver le répertoire config".to_string())?;
        let piper_dir = config_dir.join("gsp-ui").join("piper");
        std::fs::create_dir_all(&piper_dir)
            .map_err(|e| format!("Erreur création dossier piper: {}", e))?;
        Ok(piper_dir)
    }

    fn get_piper_binary_path() -> Result<std::path::PathBuf, String> {
        let piper_dir = Self::get_piper_dir()?;
        Ok(piper_dir.join("piper").join("piper"))
    }

    fn ensure_piper_binary() -> Result<std::path::PathBuf, String> {
        let binary_path = Self::get_piper_binary_path()?;

        if binary_path.exists() {
            debug!("[Piper] Binaire Piper trouvé: {:?}", binary_path);
            return Ok(binary_path);
        }

        debug!("[Piper] Téléchargement du binaire Piper...");

        // URL du binaire Piper précompilé pour Linux x86_64
        let piper_url = "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz";

        let piper_dir = Self::get_piper_dir()?;
        let tar_path = piper_dir.join("piper.tar.gz");

        download_file(piper_url, tar_path.to_str().ok_or("Chemin invalide")?)?;

        debug!("[Piper] Extraction de l'archive...");
        let file = std::fs::File::open(&tar_path)
            .map_err(|e| format!("Erreur ouverture archive: {}", e))?;
        let mut archive = flate2::read::GzDecoder::new(file);
        tar::Archive::new(&mut archive)
            .unpack(&piper_dir)
            .map_err(|e| format!("Erreur extraction: {}", e))?;

        std::fs::remove_file(&tar_path)
            .map_err(|e| format!("Erreur suppression archive: {}", e))?;

        std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Erreur permission: {}", e))?;

        debug!("[Piper] Binaire téléchargé: {:?}", binary_path);
        Ok(binary_path)
    }

    fn get_model_name(&self) -> &'static str {
        match self.lang.as_str() {
            "en" => "en_US-lessac-medium",
            _ => "fr_FR-tom-medium",
        }
    }

    fn get_models_dir() -> Result<std::path::PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Impossible de trouver le répertoire config".to_string())?;
        let models_dir = config_dir.join("gsp-ui").join("piper_models");
        std::fs::create_dir_all(&models_dir)
            .map_err(|e| format!("Erreur création dossier piper_models: {}", e))?;
        Ok(models_dir)
    }

    fn get_model_path(&self) -> Result<std::path::PathBuf, String> {
        let models_dir = Self::get_models_dir()?;
        let model_name = self.get_model_name();
        Ok(models_dir.join(format!("{}.onnx", model_name)))
    }

    fn get_model_url(model_name: &str) -> String {
        let base_url = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";
        match model_name {
            "en_US-lessac-medium" => {
                format!("{}/en/en_US/lessac/medium/en_US-lessac-medium.onnx", base_url)
            }
            "fr_FR-tom-medium" => {
                format!("{}/fr/fr_FR/tom/medium/fr_FR-tom-medium.onnx", base_url)
            }
            _ => format!("{}/{}/{}.onnx", base_url, model_name, model_name),
        }
    }

    fn ensure_model_exists(&self) -> Result<(), String> {
        let model_path = self.get_model_path()?;
        let config_path_str = format!("{}.json", model_path.display());
        let config_path = std::path::Path::new(&config_path_str);

        if model_path.exists() && config_path.exists() {
            debug!("[Piper] Modèle trouvé: {:?}", model_path);
            return Ok(());
        }

        debug!("[Piper] Téléchargement du modèle: {}", self.get_model_name());
        let model_name = self.get_model_name();
        let model_url = Self::get_model_url(model_name);
        let config_url = format!("{}.json", model_url);

        download_file(&model_url, model_path.to_str().ok_or("Chemin invalide")?)?;
        download_file(&config_url, config_path.to_str().ok_or("Chemin invalide")?)?;
        debug!("[Piper] Modèle et config téléchargés: {:?}", model_path);

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl TtsEngine for PiperTts {
    fn speak(&mut self, text: &str) -> Result<String, String> {
        self.ensure_model_exists()?;
        let piper_binary = Self::ensure_piper_binary()?;
        let model_name = self.get_model_name();
        let model_path = self.get_model_path()?;

        debug!(
            "[Piper] Synthèse vocale - texte: '{}', modèle: {}, vitesse: {}",
            text, model_name, self.speed
        );

        let length_scale = 1.0 / self.speed;

        let mut child = Command::new(&piper_binary)
            .arg("--model")
            .arg(&model_path)
            .arg("--length-scale")
            .arg(length_scale.to_string())
            .arg("--output-file")
            .arg(&self.output_file)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Erreur spawn piper: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())
                .map_err(|e| format!("Erreur écriture stdin: {}", e))?;
        }

        let result = child.wait_with_output()
            .map_err(|e| format!("Erreur attente piper: {}", e));

        match result {
            Ok(output) => {
                debug!(
                    "[Piper] Résultat piper - status: {}, stderr: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );

                if !output.status.success() {
                    return Err(format!(
                        "Erreur piper: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                debug!("[Piper] Audio généré: {}", self.output_file);
                Ok(self.output_file.clone())
            }
            Err(e) => Err(format!("Erreur lors de l'exécution de Piper: {}", e)),
        }
    }

    fn set_lang(&mut self, lang: String) {
        self.lang = lang;
    }

    fn set_speed(&mut self, speed: i32) {
        self.speed = (speed as f32 / 100.0).clamp(0.5, 3.0);
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
    fn speak(&mut self, text: &str) -> Result<String, String> {
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

fn download_file(url: &str, dest: &str) -> Result<(), String> {
    debug!("[Download] Téléchargement de: {}", url);

    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("Erreur téléchargement: {}", e))?;

    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("Erreur création fichier: {}", e))?;

    std::io::copy(&mut response.into_reader(), &mut file)
        .map_err(|e| format!("Erreur copie fichier: {}", e))?;

    debug!("[Download] Fichier téléchargé: {}", dest);
    Ok(())
}

/// Configuration du moteur espeak-ng TTS (Linux)
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct EspeakNgTts {
    lang: String,
    speed: f32,
    output_file: String,
}

#[cfg(target_os = "linux")]
impl Default for EspeakNgTts {
    fn default() -> Self {
        Self {
            lang: "fr".to_string(),
            speed: 1.0,
            output_file: "/dev/shm/out.wav".to_string(),
        }
    }
}

#[cfg(target_os = "linux")]
impl EspeakNgTts {
    pub fn with_voice(voice: String) -> Self {
        Self {
            lang: voice,
            speed: 1.0,
            output_file: "/dev/shm/out.wav".to_string(),
        }
    }

    fn get_voice(&self) -> &str {
        &self.lang
    }
}

#[cfg(target_os = "linux")]
impl TtsEngine for EspeakNgTts {
    fn speak(&mut self, text: &str) -> Result<String, String> {
        debug!(
            "[EspeakNg] Synthèse vocale - texte: '{}', voix: {}, vitesse: {}",
            text,
            self.get_voice(),
            self.speed
        );

        let speed_arg = (self.speed * 150.0) as i32;

        let mut child = Command::new("espeak-ng")
            .arg("-v")
            .arg(self.get_voice())
            .arg("-s")
            .arg(speed_arg.to_string())
            .arg("-w")
            .arg(&self.output_file)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Erreur spawn espeak-ng: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())
                .map_err(|e| format!("Erreur écriture stdin: {}", e))?;
        }

        let result = child.wait_with_output()
            .map_err(|e| format!("Erreur attente espeak-ng: {}", e));

        match result {
            Ok(output) => {
                debug!(
                    "[EspeakNg] Résultat espeak-ng - status: {}, stderr: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );

                if !output.status.success() {
                    return Err(format!(
                        "Erreur espeak-ng: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                debug!("[EspeakNg] Audio généré: {}", self.output_file);
                Ok(self.output_file.clone())
            }
            Err(e) => Err(format!("Erreur lors de l'exécution d'espeak-ng: {}", e)),
        }
    }

    fn set_lang(&mut self, _lang: String) {
        // Pour espeak-ng, la langue/voix est déjà configurée via `lang` au moment de la création
        // On ne la change pas dynamiquement
    }

    fn set_speed(&mut self, speed: i32) {
        self.speed = (speed as f32 / 100.0).clamp(0.5, 3.0);
    }
}

/// Crée une instance du moteur TTS appropriée à la plateforme et au modèle sélectionné
pub fn create_tts_engine(model_selection: Option<&str>) -> Box<dyn TtsEngine> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsTts::new())
    }
    #[cfg(target_os = "linux")]
    {
        match model_selection {
            Some(selection) if selection.starts_with("espeak-ng:") => {
                let voice = selection.strip_prefix("espeak-ng:").unwrap_or("mb-EN5");
                Box::new(EspeakNgTts::with_voice(voice.to_string()))
            }
            _ => Box::new(PiperTts::new()),
        }
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
        (playback_speed * 100.0) as i32
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PiperModelInfo {
    pub name: String,
    pub size_mb: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TtsModel {
    pub id: String,
    pub engine: String,
    pub name: String,
    pub label: String,
    pub installed: bool,
}

#[cfg(target_os = "linux")]
pub fn list_piper_models() -> Result<Vec<PiperModelInfo>, String> {
    let models_dir = PiperTts::get_models_dir()?;
    let mut models = Vec::new();

    if !models_dir.exists() {
        return Ok(models);
    }

    for entry in std::fs::read_dir(&models_dir)
        .map_err(|e| format!("Erreur lecture dossier piper_models: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Erreur lecture entry: {}", e))?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "onnx") {
            if let Ok(metadata) = std::fs::metadata(&path) {
                let file_name = entry.file_name();
                let name = file_name
                    .to_string_lossy()
                    .replace(".onnx", "")
                    .to_string();
                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                models.push(PiperModelInfo { name, size_mb });
            }
        }
    }

    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

#[cfg(target_os = "macos")]
pub fn list_piper_models() -> Result<Vec<PiperModelInfo>, String> {
    Ok(vec![])
}

#[cfg(target_os = "linux")]
pub fn delete_piper_model(model_name: &str) -> Result<(), String> {
    let models_dir = PiperTts::get_models_dir()?;

    let model_path = models_dir.join(format!("{}.onnx", model_name));
    let config_path = models_dir.join(format!("{}.json", model_name));

    if !model_path.exists() && !config_path.exists() {
        return Err(format!("Le modèle '{}' n'existe pas", model_name));
    }

    if model_path.exists() {
        std::fs::remove_file(&model_path)
            .map_err(|e| format!("Erreur suppression modèle {}: {}", model_name, e))?;
    }

    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Erreur suppression config {}: {}", model_name, e))?;
    }

    debug!("[Piper] Modèle '{}' supprimé avec succès", model_name);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn delete_piper_model(model_name: &str) -> Result<(), String> {
    Err("Suppression des modèles Piper non supportée sur macOS".to_string())
}

#[cfg(target_os = "linux")]
pub fn download_piper_model(model_name: &str) -> Result<(), String> {
    let models_dir = PiperTts::get_models_dir()?;

    if models_dir.join(format!("{}.onnx", model_name)).exists() {
        return Err(format!("Le modèle '{}' est déjà installé", model_name));
    }

    debug!("[Piper] Téléchargement du modèle: {}", model_name);

    let model_url = PiperTts::get_model_url(model_name);
    let config_url = format!("{}.json", model_url);

    let model_path = models_dir.join(format!("{}.onnx", model_name));
    let config_path = models_dir.join(format!("{}.json", model_name));

    download_file(&model_url, model_path.to_str().ok_or("Chemin invalide")?)?;
    download_file(&config_url, config_path.to_str().ok_or("Chemin invalide")?)?;

    debug!("[Piper] Modèle '{}' téléchargé avec succès", model_name);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn download_piper_model(_model_name: &str) -> Result<(), String> {
    Err("Téléchargement des modèles Piper non supporté sur macOS".to_string())
}

fn is_espeak_ng_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("espeak-ng")
            .arg("--version")
            .output()
            .ok()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn get_espeak_ng_available_voices() -> Result<Vec<String>, String> {
    let output = Command::new("espeak-ng")
        .arg("--voices")
        .output()
        .map_err(|e| format!("Erreur lecture voix espeak-ng: {}", e))?;

    if !output.status.success() {
        return Err("Impossible de lister les voix espeak-ng".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let lang_code = parts[1].trim();
            if !lang_code.is_empty() && lang_code != "Language" {
                voices.push(lang_code.to_string());
            }
        }
    }

    debug!("[EspeakNg] Voix disponibles: {:?}", voices);
    Ok(voices)
}

#[cfg(target_os = "linux")]
fn get_mbrola_voices_installed() -> Vec<String> {
    let mut mbrola_voices = Vec::new();

    let paths = vec![
        std::path::PathBuf::from("/usr/share/mbrola"),
        std::path::PathBuf::from("/usr/local/share/mbrola"),
    ];

    for base_path in paths {
        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        let voice_id = format!("mb-{}", dir_name.to_uppercase());
                        if !mbrola_voices.contains(&voice_id) {
                            mbrola_voices.push(voice_id);
                        }
                    }
                }
            }
        }
    }

    mbrola_voices.sort();
    debug!("[EspeakNg] Voix mbrola installées: {:?}", mbrola_voices);
    mbrola_voices
}

pub fn get_available_tts_models() -> Result<Vec<TtsModel>, String> {
    let mut models = Vec::new();

    // Piper models
    let piper_models = vec![
        ("fr_FR-tom-medium", "Piper: Français (Tom)"),
        ("en_US-lessac-medium", "Piper: English (Lessac)"),
    ];

    for (model_name, label) in piper_models {
        let id = format!("piper:{}", model_name);
        #[cfg(target_os = "linux")]
        let installed = {
            let models_dir = PiperTts::get_models_dir().ok();
            let path = models_dir
                .as_ref()
                .map(|d| d.join(format!("{}.onnx", model_name)));
            path.map(|p| p.exists()).unwrap_or(false)
        };

        #[cfg(target_os = "macos")]
        let installed = false;

        models.push(TtsModel {
            id,
            engine: "piper".to_string(),
            name: model_name.to_string(),
            label: label.to_string(),
            installed,
        });
    }

    // espeak-ng voices - détection dynamique
    #[cfg(target_os = "linux")]
    {
        if is_espeak_ng_available() {
            match get_espeak_ng_available_voices() {
                Ok(available_voices) => {
                    let mbrola_installed = get_mbrola_voices_installed();

                    // Voix mbrola installées
                    for mbrola_voice in mbrola_installed {
                        let id = format!("espeak-ng:{}", mbrola_voice);
                        let label = format!("eSpeak-NG: {} (Multibande)", mbrola_voice);

                        models.push(TtsModel {
                            id,
                            engine: "espeak-ng".to_string(),
                            name: mbrola_voice,
                            label,
                            installed: true,
                        });
                    }

                    // Voix espeak intégrées (fr, en-us, en-gb, etc.)
                    for voice in available_voices {
                        if (voice.contains("fr") || voice.contains("en")) && !voice.starts_with("mb-") {
                            let id = format!("espeak-ng:{}", voice);
                            let label = format!("eSpeak-NG: {} (builtin)", voice);

                            models.push(TtsModel {
                                id,
                                engine: "espeak-ng".to_string(),
                                name: voice,
                                label,
                                installed: true,
                            });
                        }
                    }
                }
                Err(e) => {
                    debug!("[EspeakNg] Erreur lors de la lecture des voix: {}", e);
                }
            }
        }
    }

    Ok(models)
}
