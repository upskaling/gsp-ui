//! Utilitaires de traitement de texte pour GSP
//!
//! Fournit des fonctions pour le nettoyage et le formatage du texte.

use log::warn;
use std::collections::HashMap;
use std::path::Path;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

/// Parse les hashtags en séparant les mots CamelCase
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::parse_hashtag;
/// let text = String::from("#helloWorld");
/// let result = parse_hashtag(&text);
/// assert_eq!(result, "#hello World");
/// ```
pub fn parse_hashtag(string: &str) -> String {
    let mut result = String::with_capacity(string.len() + 16);

    for word in string.split_whitespace() {
        if word.starts_with('#') && word.len() > 1 {
            // Traitement spécial pour les hashtags
            result.push('#');

            let mut chars = word[1..].chars().peekable();
            let mut current_word = String::new();

            while let Some(c) = chars.next() {
                if c.is_uppercase() {
                    // Si on a déjà un mot en cours et que le prochain caractère
                    // est minuscule, c'est le début d'un nouveau mot
                    if !current_word.is_empty() && chars.peek().is_some_and(|n| n.is_lowercase()) {
                        result.push_str(&current_word);
                        result.push(' ');
                        current_word.clear();
                    }
                    current_word.push(c);
                } else {
                    current_word.push(c);
                }
            }

            // Ajouter le dernier mot
            if !current_word.is_empty() {
                result.push_str(&current_word);
            }
        } else {
            result.push_str(word);
        }
        result.push(' ');
    }

    trim_whitespace(&result)
}

/// Supprime les espaces multiples dans une chaîne
/// https://stackoverflow.com/a/71864249
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::trim_whitespace;
/// let text = "  a  b  c  ";
/// let result = trim_whitespace(text);
/// assert_eq!(result, "a b c");
/// ```
pub fn trim_whitespace(string: &str) -> String {
    let mut result = String::with_capacity(string.len());
    let mut words = string.split_whitespace().peekable();

    while let Some(word) = words.next() {
        result.push_str(word);
        if words.peek().is_some() {
            result.push(' ');
        }
    }

    result
}

/// Supprime les caractères spéciaux et normalise le texte
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::remove_special_characters;
/// let text = "𝐠𝐫𝐚𝐬 et 𝘪𝘵𝘢𝘭𝘪𝘤";
/// let result = remove_special_characters(text);
/// assert_eq!(result, "gras et italic");
/// ```
pub fn remove_special_characters(string: &str) -> String {
    let mapping = create_character_mapping();

    string
        .chars()
        .map(|c| mapping.get(&c).copied().unwrap_or(c))
        .collect()
}

/// Normalise le texte (NFC) et supprime les caractères invisibles
///
/// Élimine les sélecteurs de variation, les caractères de format
/// (zero-width, contrôles bidi, ...), les espaces insécables et les
/// diacritiques combinants qui subsistent après la normalisation NFC.
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::sanitize_unicode;
/// let text = "C\u{FE01}h\u{FE01}urc\u{FE01}h";
/// let result = sanitize_unicode(text);
/// assert_eq!(result, "Church");
/// ```
pub fn sanitize_unicode(text: &str) -> String {
    let normalized: String = text.nfc().collect();
    let mut result = String::with_capacity(normalized.len());

    for c in normalized.chars() {
        match c {
            // Espaces insécables et séparateurs -> espace simple
            '\u{00A0}' | '\u{2007}' | '\u{202F}' | '\u{2028}' | '\u{2029}' => {
                result.push(' ');
            }
            _ if is_invisible(c) || is_combining_mark(c) => {}
            _ => result.push(c),
        }
    }

    result
}

/// Indique si un caractère est invisible (format, sélecteur de variation, ...)
fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{0000}'..='\u{0008}'      // caractères de contrôle (hors \t et \n)
        | '\u{000B}'..='\u{001F}'    // caractères de contrôle
        | '\u{007F}'                  // DEL
        | '\u{00AD}'                  // soft hyphen
        | '\u{034F}'                  // combining grapheme joiner
        | '\u{061C}'                  // marque directionnelle arabe
        | '\u{115F}'..='\u{1160}'     // remplisseurs hangul
        | '\u{17B4}'..='\u{17B5}'     // voyelles inhérentes khmères
        | '\u{180E}'                  // séparateur de voyelle mongol
        | '\u{200B}'..='\u{200F}'     // zero-width, ZWNJ, ZWJ, LRM, RLM
        | '\u{202A}'..='\u{202E}'     // contrôles bidirectionnels
        | '\u{2060}'..='\u{2064}'     // word joiner, opérateurs invisibles
        | '\u{2066}'..='\u{2069}'     // isolats bidirectionnels
        | '\u{206A}'..='\u{206F}'     // formatage déprécié
        | '\u{FE00}'..='\u{FE0F}'     // sélecteurs de variation
        | '\u{FEFF}'                   // BOM / zero-width no-break space
        | '\u{FFF9}'..='\u{FFFB}'     // annotations interlinéaires
        | '\u{E0000}'..='\u{E007F}'   // tag characters (drapeaux)
        | '\u{E0100}'..='\u{E01EF}'   // sélecteurs de variation supplémentaires
    )
}

/// supprimer les caractères markdown
///
/// # Examples
/// ```
/// # use gsp_ui_lib::textutils::remove_markdown;
/// let text = "**Hello World**";
/// let result = remove_markdown(text);
/// assert_eq!(result, "Hello World");
/// ```
pub fn remove_markdown(string: &str) -> String {
    let mut result = String::with_capacity(string.len());
    let mut chars = string.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                // Gérer les séquences de * consécutifs
                while let Some(next) = chars.peek() {
                    if *next == '*' {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            '_' => {
                // Ignorer les underscores (italique alternatif)
            }
            _ => result.push(c),
        }
    }

    result
}

/// Lit et formate les variables (snake_case, kebab-case, CamelCase)
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::read_vars;
/// let text = "HelloWorld";
/// let result = read_vars(text);
/// assert_eq!(result, "Hello World");
/// ```
pub fn read_vars(string: &str) -> String {
    let mut result = String::with_capacity(string.len() + 16);

    // Première passe : remplacer les séparateurs
    for c in string.chars() {
        match c {
            '_' | '-' => result.push(' '),
            _ => result.push(c),
        }
    }

    // Deuxième passe : gérer le CamelCase
    result = parse_camel_case(&result);

    // Enlever les marqueurs markdown
    result = remove_markdown(&result);

    // Normaliser les espaces
    trim_whitespace(&result)
}

/// Parse le CamelCase en séparant les mots
fn parse_camel_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 8);
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() && !result.is_empty() {
            // Vérifier si le caractère suivant est minuscule pour éviter
            // de couper les acronymes (ex: "HTML" -> reste "HTML")
            if let Some(next) = chars.peek() {
                if next.is_lowercase() {
                    result.push(' ');
                }
            } else {
                result.push(' ');
            }
        }
        result.push(c);
    }

    result
}

/// Crée un mapping efficace des caractères spéciaux
fn create_character_mapping() -> HashMap<char, char> {
    let mut mapping = HashMap::with_capacity(200);

    // Caractères de quotes à remplacer par des espaces
    for &c in &['"', '\'', '`', '[', ']', '{', '}'] {
        mapping.insert(c, ' ');
    }

    // Caractères spéciaux spécifiques
    let replacements = [
        ('ᴀ', 'a'),
        ('ʙ', 'b'),
        ('ᴄ', 'c'),
        ('ᴅ', 'd'),
        ('ᴇ', 'e'),
        ('ɢ', 'g'),
        ('ʜ', 'h'),
        ('ɪ', 'i'),
        ('ᴊ', 'j'),
        ('ᴋ', 'k'),
        ('ʟ', 'l'),
        ('ᴍ', 'm'),
        ('ɴ', 'n'),
        ('ᴏ', 'o'),
        ('ᴘ', 'p'),
        ('ʀ', 'r'),
        ('ᴛ', 't'),
        ('ᴜ', 'u'),
        ('ᴠ', 'v'),
        ('ᴡ', 'w'),
        ('ʏ', 'y'),
        ('ᴢ', 'z'),
        ('а', 'a'),
        ('А', 'A'),
        ('в', 'b'),
        ('В', 'b'),
        ('г', 'r'),
        ('ғ', 'f'),
        ('е', 'e'),
        ('Е', 'E'),
        ('ѕ', 's'),
        ('Ѕ', 'S'),
        ('и', 'n'),
        ('і', 'i'),
        ('І', 'I'),
        ('к', 'k'),
        ('К', 'k'),
        ('л', 'n'),
        ('м', 'm'),
        ('М', 'm'),
        ('н', 'h'),
        ('Н', 'H'),
        ('о', 'o'),
        ('О', 'O'),
        ('р', 'p'),
        ('Р', 'p'),
        ('с', 'c'),
        ('С', 'C'),
        ('т', 't'),
        ('Т', 'T'),
        ('у', 'y'),
        ('х', 'x'),
    ];

    for (from, to) in replacements {
        mapping.insert(from, to);
    }

    // Alphabet stylisé
    for &base in &[
        '𝐀', '𝐚', '𝑨', '𝒂', '𝒜', '𝒶', '𝔸', '𝕒', '𝕬', '𝖆', '𝖠', '𝖺', '𝗔', '𝗮', '𝘈', '𝘢', '𝘼', '𝙖',
        '𝙰', '𝚊', '𝐴', '𝑎', '𝔄', '𝔞', '𝓐', '𝓪', 'Ａ', 'ａ', '🅰', '🅐',
    ] {
        for i in 0..26 {
            if let (Some(from), Some(to)) = (
                std::char::from_u32(base as u32 + i),
                std::char::from_u32('a' as u32 + i),
            ) {
                mapping.insert(from, to);
            }
        }
    }

    // Chiffres stylisés
    for &base in &['𝟎', '𝟘', '𝟢', '𝟬', '𝟶'] {
        for i in 0..10 {
            if let (Some(from), Some(to)) = (
                std::char::from_u32(base as u32 + i),
                std::char::from_u32('0' as u32 + i),
            ) {
                mapping.insert(from, to);
            }
        }
    }

    mapping
}

/// Prétraite le texte (nettoyage complet avant traitement)
///
/// Applique les transformations dans cet ordre:
/// 1. Normalise Unicode et supprime les caractères invisibles
/// 2. Parse les hashtags CamelCase
/// 3. Normalise les espaces
/// 4. Supprime les caractères spéciaux
///
/// # Exemples
/// ```
/// # use gsp_ui_lib::textutils::preprocess_text;
/// let text = "  #helloWorld  est  𝐠𝐫𝐚𝐬  ";
/// let result = preprocess_text(text);
/// assert_eq!(result, "#hello World est gras");
/// ```
pub fn preprocess_text(text: &str) -> String {
    let mut text = sanitize_unicode(text);
    text = parse_hashtag(&text);
    text = trim_whitespace(&text);
    text = remove_special_characters(&text);
    text
}

/// Remplace les termes selon le dictionnaire
pub fn replace(text: &str) -> String {
    static DICT_PATH: &str = "dict/fr_FR";

    if !Path::new(DICT_PATH).exists() {
        return text.to_string();
    }

    let replacements = match load_replacements() {
        Ok(map) => map,
        Err(e) => {
            warn!(
                "[DICT] Avertissement: impossible de charger le dictionnaire: {}",
                e
            );
            return text.to_string();
        }
    };

    if replacements.is_empty() {
        return text.to_string();
    }

    // Appliquer les remplacements de manière efficace
    let mut result = text.to_string();
    for (from, to) in replacements {
        result = result.replace(&from, &to);
    }

    result
}

/// Charge la liste des remplacements depuis le dictionnaire
fn load_replacements() -> Result<HashMap<String, String>, std::io::Error> {
    let mut replacements = HashMap::new();
    let files = list_files("dict/fr_FR")?;

    for file in files {
        let metadata = std::fs::metadata(&file)?;
        if !metadata.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&file)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // Ignorer les commentaires et lignes vides
            }

            if let Some((key, value)) = line.split_once('=') {
                replacements.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    Ok(replacements)
}

/// Liste tous les fichiers d'un répertoire
fn list_files(path: &str) -> Result<Vec<String>, std::io::Error> {
    let mut result = Vec::new();
    let entries = std::fs::read_dir(path)?;

    for entry in entries {
        let path = entry?.path();
        if let Some(path_str) = path.to_str() {
            result.push(path_str.to_string());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hashtag() {
        let result = parse_hashtag("#helloWorld");
        assert_eq!(result, "#hello World");
    }

    #[test]
    fn test_parse_hashtag2() {
        let result = parse_hashtag("le #chat est #mignon");
        assert_eq!(result, "le #chat est #mignon");
    }

    #[test]
    fn test_parse_hashtag_multiple_uppercase() {
        let result = parse_hashtag("#HTML5Test");
        assert_eq!(result, "#HTML5 Test");
    }

    #[test]
    fn test_parse_hashtag_with_numbers() {
        let result = parse_hashtag("#CSS3Animation");
        assert_eq!(result, "#CSS3 Animation");
    }

    #[test]
    fn test_parse_hashtag_multiple_words() {
        let result = parse_hashtag("#helloWorld and #goodbyeMoon");
        assert_eq!(result, "#hello World and #goodbye Moon");
    }

    #[test]
    fn test_trim_whitespace() {
        let result = trim_whitespace("  a  b  c  ");
        assert_eq!(result, "a b c");
    }

    #[test]
    fn test_remove_special_characters() {
        let result = remove_special_characters("𝐠𝐫𝐚𝐬 et 𝘪𝘵𝘢𝘭𝘪𝘤");
        assert_eq!(result, "gras et italic");
    }

    #[test]
    fn test_remove_special_characters_with_alphabet() {
        let result = remove_special_characters("𝐀𝐚𝑨𝒂𝒜𝒶𝔸𝕒𝕬𝖆𝖠𝖺𝗔𝗮𝘈𝘢𝘼𝙖𝙰𝚊𝐴𝑎𝔄𝔞𝓐𝓪Ａａ🅰🅐");
        assert_eq!(result, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    }

    #[test]
    fn test_remove_special_characters_with_alphabet_stylised() {
        let result = remove_special_characters("🅰🅱🅲🅳🅴🅵🅶🅷🅸🅹🅺🅻🅼🅽🅾🅿🆀🆁🆂🆃🆄🆅🆆🆇🆈🆉");
        assert_eq!(result, "abcdefghijklmnopqrstuvwxyz");

        let result = remove_special_characters("🅐🅑🅒🅓🅔🅕🅖🅗🅘🅙🅚🅛🅜🅝🅞🅟🅠🅡🅢🅣🅤🅥🅦🅧🅨🅩");
        assert_eq!(result, "abcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn test_remove_special_characters_with_numbers() {
        let result = remove_special_characters("𝟎𝟘𝟢𝟬𝟶");
        assert_eq!(result, "00000");
    }

    #[test]
    fn test_replace_special_chars() {
        let result = remove_special_characters("С і І ѕ");
        assert_eq!(result, "C i I s");
    }

    #[test]
    fn test_sanitize_unicode_variation_selectors() {
        let dirty = "Phileas Fogg comp︁tait quatre-vingts jours pour accompli︌r son tour du monde et quel︊ques he︆ures de sommeil. I︍l concédait qu'un voya︊geur pati︌ent y serait par︇venu dès 1872. Passepa︀rtout, qui ten︊ait le regis︋tre des bag︈ages, range︈ait désorm︄ais l'explo︇it parmi les gran︁des aventures.";
        let expected = "Phileas Fogg comptait quatre-vingts jours pour accomplir son tour du monde et quelques heures de sommeil. Il concédait qu'un voyageur patient y serait parvenu dès 1872. Passepartout, qui tenait le registre des bagages, rangeait désormais l'exploit parmi les grandes aventures.";

        let result = sanitize_unicode(dirty);

        assert_eq!(result, expected);
        assert!(
            !result
                .chars()
                .any(|c| ('\u{FE00}'..='\u{FE0F}').contains(&c)),
            "sélecteurs de variation résiduels"
        );
    }

    #[test]
    fn test_sanitize_unicode_normalization_nfc() {
        assert_eq!(sanitize_unicode("e\u{0301}xecute"), "éxecute");
        assert_eq!(sanitize_unicode("éxecute"), "éxecute");
        assert_eq!(sanitize_unicode("cafe\u{0301}"), "café");
    }

    #[test]
    fn test_sanitize_unicode_zero_width_and_bidi() {
        assert_eq!(sanitize_unicode("bon\u{200B}jour"), "bonjour");
        assert_eq!(sanitize_unicode("bo\u{200D}njour"), "bonjour");
        assert_eq!(sanitize_unicode("\u{202E}rtl"), "rtl");
        assert_eq!(sanitize_unicode("\u{FEFF}texte"), "texte");
        assert_eq!(sanitize_unicode("mot\u{00AD}hyphen"), "mothyphen");
        assert_eq!(sanitize_unicode("a\u{00A0}b\u{202F}c"), "a b c");
    }

    #[test]
    fn test_preprocess_text_with_invisible_characters() {
        let dirty =
            "Phileas Fogg comp︁tait quatre-vingts jours pour accompli︌r son tour du monde et quel︊ques he︆ures de sommeil.";
        let result = preprocess_text(dirty);

        assert_eq!(
            result,
            "Phileas Fogg comptait quatre-vingts jours pour accomplir son tour du monde et quelques heures de sommeil."
        );
    }

    #[test]
    fn test_read_vars() {
        let result = read_vars("HelloWorld");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_read_vars_snake_case() {
        let result = read_vars("hello_world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_read_vars_kebab_case() {
        let result = read_vars("hello-world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_remove_markdown() {
        let result = remove_markdown("**Hello** *World*");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_parse_camel_case() {
        let result = parse_camel_case("HelloWorld");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_parse_camel_case_acronym() {
        let result = parse_camel_case("HTML5Parser");
        assert_eq!(result, "HTML5 Parser");
    }

    #[test]
    fn test_replace_no_dict() {
        let result = replace("HelloWorld");
        assert_eq!(result, "HelloWorld");
    }
}
