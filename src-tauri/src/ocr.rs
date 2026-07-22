use image::GenericImageView;
use log::error;
use std::fs;
use std::io;
use std::path::PathBuf;
use tesseract_rs::TesseractAPI;

fn tessdata_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TESSDATA_PREFIX") {
        return PathBuf::from(dir);
    }
    let home = dirs::home_dir().expect("home directory not found");
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/tesseract-rs/tessdata")
    } else if cfg!(target_os = "linux") {
        home.join(".tesseract-rs/tessdata")
    } else {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("tesseract-rs/tessdata")
    }
}

fn download_lang(lang: &str) -> bool {
    let dir = tessdata_dir();
    let path = dir.join(format!("{lang}.traineddata"));
    if path.exists() {
        return true;
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        error!("[OCR] Erreur création dossier tessdata: {e}");
        return false;
    }

    let url = format!("https://github.com/tesseract-ocr/tessdata_best/raw/main/{lang}.traineddata");
    match ureq::get(&url).call() {
        Ok(response) => {
            let mut reader = response.into_reader();
            let mut file = match fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    error!("[OCR] Erreur écriture {lang}: {e}");
                    return false;
                }
            };
            if let Err(e) = io::copy(&mut reader, &mut file) {
                error!("[OCR] Erreur téléchargement {lang}: {e}");
                false
            } else {
                true
            }
        }
        Err(_) => {
            let fallback = format!("https://github.com/tesseract-ocr/tessdata_fast/raw/main/{lang}.traineddata");
            match ureq::get(&fallback).call() {
                Ok(response) => {
                    let mut reader = response.into_reader();
                    let mut file = match fs::File::create(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            error!("[OCR] Erreur écriture {lang}: {e}");
                            return false;
                        }
                    };
                    if let Err(e) = io::copy(&mut reader, &mut file) {
                        error!("[OCR] Erreur téléchargement {lang}: {e}");
                        false
                    } else {
                        true
                    }
                }
                Err(e) => {
                    error!("[OCR] Erreur téléchargement {lang}: {e}");
                    false
                }
            }
        }
    }
}

pub fn tesseract(screenshot_path: &str, lang: &str) -> String {
    let code = match lang {
        "de-DE" => "deu",
        "en-GB" => "eng",
        "es-ES" => "spa",
        "fr-FR" => "fra",
        "it-IT" => "ita",
        _ => "eng+fra",
    };

    for l in code.split('+') {
        if !download_lang(l) {
            error!("[OCR] Langue {l} indisponible");
            return String::new();
        }
    }

    let img = match image::open(screenshot_path) {
        Ok(img) => img,
        Err(e) => {
            error!("[OCR] Erreur lecture image: {e}");
            return String::new();
        }
    };

    let (width, height) = img.dimensions();
    let gray = img.to_luma8();
    let data = gray.as_raw();

    let data_dir = tessdata_dir();
    let api = TesseractAPI::new();
    if let Err(e) = api.init(&data_dir, code) {
        error!("[OCR] Erreur init Tesseract: {e}");
        return String::new();
    }

    if let Err(e) = api.set_image(data, width as i32, height as i32, 1, width as i32) {
        error!("[OCR] Erreur set_image: {e}");
        return String::new();
    }

    let _ = api.set_source_resolution(300);

    if let Err(e) = api.recognize() {
        error!("[OCR] Erreur recognize: {e}");
        return String::new();
    }

    api.get_utf8_text().unwrap_or_else(|e| {
        error!("[OCR] Erreur get_utf8_text: {e}");
        String::new()
    })
}
