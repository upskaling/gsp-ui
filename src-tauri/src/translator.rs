//! Traduction locale avec translateLocally
//!
//! Fournit une traduction local en utilisant l'outil translateLocally.

use log::info;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize, Debug)]
struct TranslateRequest {
    id: i32,
    command: String,
    data: TranslateRequestData,
}

#[derive(Serialize, Deserialize, Debug)]
struct TranslateRequestData {
    src: String,
    trg: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TranslateResponse {
    id: i32,
    success: bool,
    data: Option<TranslateResponseData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TranslateResponseData {
    target: TranslateResponseTargetData,
}

#[derive(Serialize, Deserialize, Debug)]
struct TranslateResponseTargetData {
    text: String,
}

/// Vérifie si la commande translateLocally est disponible
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Traduit un texte d'une langue source vers une langue cible
///
/// # Arguments
/// * `text` - Le texte à traduire
/// * `lang_from` - Code de langue source (ex: "en")
/// * `lang_to` - Code de langue cible (ex: "fr")
///
/// # Retour
/// Retourne le texte traduit ou une erreur
pub fn translate(text: &str, lang_from: &str, lang_to: &str) -> Result<String, String> {
    if !command_exists("translateLocally") {
        return Err("translateLocally n'est pas disponible".to_string());
    }

    let mut command = Command::new("translateLocally")
        .arg("-p")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Erreur lors du lancement de translateLocally: {}", e))?;

    let mut stdin = command
        .stdin
        .take()
        .ok_or("Erreur: impossible d'ouvrir stdin")?;
    let mut stdout = command
        .stdout
        .take()
        .ok_or("Erreur: impossible d'ouvrir stdout")?;

    let request = TranslateRequest {
        id: 1,
        command: "Translate".to_string(),
        data: TranslateRequestData {
            src: lang_from[..2.min(lang_from.len())].to_string(),
            trg: lang_to[..2.min(lang_to.len())].to_string(),
            text: text.to_string(),
        },
    };

    let request_bytes = serde_json::to_vec(&request)
        .map_err(|e| format!("Erreur de sérialisation de la requête: {}", e))?;

    let length = (request_bytes.len() as u32).to_ne_bytes();

    stdin
        .write_all(&length)
        .map_err(|e| format!("Erreur lors de l'écriture de la longueur: {}", e))?;

    stdin
        .write_all(&request_bytes)
        .map_err(|e| format!("Erreur lors de l'écriture de la requête: {}", e))?;

    // Fermer stdin pour indiquer que nous avons fini d'écrire
    drop(stdin);

    let mut response_len = [0u8; 4];
    stdout.read_exact(&mut response_len).map_err(|e| {
        format!(
            "Erreur lors de la lecture de la longueur de la réponse: {}",
            e
        )
    })?;

    let response_len = u32::from_ne_bytes(response_len);

    let mut response_bytes = vec![0u8; response_len as usize];
    stdout
        .read_exact(&mut response_bytes)
        .map_err(|e| format!("Erreur lors de la lecture de la réponse: {}", e))?;

    let response = serde_json::from_slice::<TranslateResponse>(&response_bytes)
        .map_err(|e| format!("Erreur de désérialisation de la réponse: {}", e))?;

    // Attendre que le processus se termine proprement
    let _ = command.wait();

    if let Some(data) = response.data {
        info!(
            "[TRANSLATOR] Traduction réussie: {} -> {}",
            lang_from, lang_to
        );
        Ok(data.target.text)
    } else {
        Err("La réponse ne contient pas de données de traduction".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_request_serialization() {
        let request = TranslateRequest {
            id: 1,
            command: "Translate".to_string(),
            data: TranslateRequestData {
                src: "en".to_string(),
                trg: "fr".to_string(),
                text: "Hello".to_string(),
            },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let expected =
            r#"{"id":1,"command":"Translate","data":{"src":"en","trg":"fr","text":"Hello"}}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_translate_response_deserialization() {
        let json_data = r#"
        {
            "id": 1,
            "success": true,
            "data": {
                "target": {
                    "text": "Bonjour"
                }
            }
        }"#;

        let response: TranslateResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.success);
        assert_eq!(response.data.unwrap().target.text, "Bonjour");
    }

    #[test]
    fn test_translate_response_no_data() {
        let json_data = r#"
        {
            "id": 1,
            "success": false,
            "data": null
        }"#;

        let response: TranslateResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.id, 1);
        assert!(!response.success);
        assert!(response.data.is_none());
    }
}
