//! Prise en charge de l'alphabet phonétique international (API / IPA)
//!
//! Les moteurs de synthèse utilisés (Piper, `say`) ne savent pas prononcer
//! directement les symboles de l'API : ils liraient « ʃ » ou « ɔ̃ » comme des
//! caractères inconnus, voire les ignoreraient.
//!
//! Ce module détecte les transcriptions phonétiques dans un texte et les
//! convertit en une graphie prononçable dans la langue de lecture. Le moteur
//! TTS phonémise ensuite normalement cette graphie.
//!
//! # Exemples
//! ```
//! # use gsp_ui_lib::ipa::{contains_ipa, to_pronounceable};
//! assert!(contains_ipa("Le mot /ʃa/ est court"));
//! assert!(!contains_ipa("Le chat est petit"));
//! assert_eq!(to_pronounceable("/bɔ̃ʒuʁ/", "fr"), "bonjour");
//! ```

/// Caractères qui n'apparaissent pratiquement jamais dans un texte français ou
/// anglais courant et qui signalent donc une transcription phonétique.
///
/// Les lettres `œ`, `æ`, `ç`, `ñ` et les voyelles accentuées sont volontairement
/// exclues : elles font partie du texte normal.
const IPA_MARKERS: &[char] = &[
    // Voyelles
    'ə', 'ɚ', 'ɛ', 'ɔ', 'ɜ', 'ɞ', 'ɘ', 'ɵ', 'ɤ', 'ʌ', 'ʊ', 'ɨ', 'ʉ', 'ɯ', 'ɐ', 'ɑ', 'ɒ', 'ɪ', 'ʏ',
    'ɶ', 'ɝ', 'ᵻ', // Consonnes
    'ʃ', 'ʒ', 'ɲ', 'ŋ', 'θ', 'ð', 'ʁ', 'ʀ', 'ɾ', 'ɽ', 'ɹ', 'ɻ', 'ɰ', 'ɥ', 'ʍ', 'ɬ', 'ɮ', 'ʋ', 'ɓ',
    'ɗ', 'ʄ', 'ɠ', 'ʛ', 'ʔ', 'ʕ', 'ʢ', 'ʡ', 'ǀ', 'ǁ', 'ǂ', 'ǃ', 'ɣ', 'ʝ', 'ɟ', 'ɢ', 'ʜ', 'ɦ', 'ɱ',
    'ɳ', 'ɴ', 'ɺ', 'ʙ', 'ⱱ', 'ɡ', 'ᵑ', // Durée, accent, tons et diacritiques
    'ˈ', 'ˌ', 'ː', 'ˑ', 'ʰ', 'ʲ', 'ʷ', 'ˠ', 'ˤ', '˥', '˦', '˧', '˨', '˩', '͡', '‿',
];

/// Indique si un caractère est spécifique à l'API.
fn is_ipa_marker(c: char) -> bool {
    IPA_MARKERS.contains(&c)
}

/// Indique si un caractère est un diacritique combinant.
fn is_combining(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// Détecte la présence d'une transcription en alphabet phonétique international.
///
/// La détection se base sur les symboles propres à l'API. Un texte ordinaire
/// contenant des lettres accentuées (é, œ, ñ, …) n'est donc pas détecté.
pub fn contains_ipa(text: &str) -> bool {
    text.chars().any(is_ipa_marker)
}

/// Convertit les transcriptions phonétiques d'un texte en graphie prononçable,
/// en laissant intact le reste du texte.
///
/// * `lang` : langue de lecture (`fr` par défaut, `en` pour l'anglais).
pub fn to_pronounceable(text: &str, lang: &str) -> String {
    let table = if lang.to_ascii_lowercase().starts_with("en") {
        EN
    } else {
        FR
    };

    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut word = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Transcription délimitée : /.../ ou [...]
        if c == '/' || c == '[' {
            let close = if c == '/' { '/' } else { ']' };

            if let Some(offset) = chars[i + 1..].iter().position(|&x| x == close) {
                let inner: String = chars[i + 1..i + 1 + offset].iter().collect();

                if contains_ipa(&inner) {
                    flush_word(&mut result, &mut word, table);
                    result.push_str(&convert_ipa_token(&inner, table));
                    i += offset + 2;
                    continue;
                }
            }
        }

        if c.is_whitespace() {
            flush_word(&mut result, &mut word, table);
            result.push(c);
        } else {
            word.push(c);
        }

        i += 1;
    }

    flush_word(&mut result, &mut word, table);
    result
}

/// Ajoute un mot au résultat en le convertissant seulement s'il contient de l'API.
fn flush_word(result: &mut String, word: &mut String, table: &[(&str, &str)]) {
    if word.is_empty() {
        return;
    }

    if contains_ipa(word) {
        result.push_str(&convert_ipa_token(word, table));
    } else {
        result.push_str(word);
    }

    word.clear();
}

/// Convertit un fragment de transcription en graphie prononçable.
fn convert_ipa_token(token: &str, table: &[(&str, &str)]) -> String {
    let chars: Vec<char> = token.chars().collect();
    let mut result = String::with_capacity(token.len());
    let mut i = 0;

    while i < chars.len() {
        // Voyelle nasale : voyelle + tilde combinant (ex. « ɔ̃ »).
        if i + 1 < chars.len() && chars[i + 1] == '\u{0303}' {
            let key: String = [chars[i], '\u{0303}'].iter().collect();
            if let Some(replacement) = lookup(table, &key) {
                result.push_str(replacement);
                i += 2;
                continue;
            }
        }

        // On essaie les séquences les plus longues d'abord (diphtongues,
        // affriquées, …) avant de retomber sur un symbole isolé.
        let mut matched = false;
        for len in (1..=3).rev() {
            if i + len > chars.len() {
                continue;
            }

            let key: String = chars[i..i + len].iter().collect();
            if let Some(replacement) = lookup(table, &key) {
                result.push_str(replacement);
                i += len;
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        let c = chars[i];
        // Les symboles inconnus de l'API sont ignorés ; la ponctuation et les
        // chiffres éventuels sont conservés.
        if !is_combining(c) && !is_ipa_marker(c) {
            result.push(c);
        }

        i += 1;
    }

    result
}

fn lookup<'a>(table: &[(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    table
        .iter()
        .find(|(pattern, _)| *pattern == key)
        .map(|(_, replacement)| *replacement)
}

/// Correspondances API → graphie française.
const FR: &[(&str, &str)] = &[
    // Voyelles nasales
    ("ɑ̃", "an"),
    ("ɛ̃", "in"),
    ("ɔ̃", "on"),
    ("œ̃", "un"),
    ("a\u{0303}", "an"),
    ("e\u{0303}", "in"),
    ("o\u{0303}", "on"),
    ("u\u{0303}", "oun"),
    // Diphtongues (transcriptions anglaises lues en français)
    ("aɪə", "aïeur"),
    ("aʊə", "aoueur"),
    ("eɪə", "éieur"),
    ("ɔɪə", "oyeur"),
    ("aɪ", "aï"),
    ("aʊ", "aou"),
    ("eɪ", "é"),
    ("oʊ", "o"),
    ("əʊ", "o"),
    ("ɔɪ", "oy"),
    ("ɪə", "ièr"),
    ("eə", "èr"),
    ("ʊə", "our"),
    // Affriquées
    ("t͡ʃ", "tch"),
    ("d͡ʒ", "dj"),
    ("tʃ", "tch"),
    ("dʒ", "dj"),
    ("ʦ", "ts"),
    ("ʣ", "dz"),
    // Voyelles
    ("i", "i"),
    ("y", "u"),
    ("ɪ", "i"),
    ("ʏ", "u"),
    ("e", "é"),
    ("ø", "eu"),
    ("œ", "eu"),
    ("ɛ", "è"),
    ("ɜ", "eu"),
    ("ə", "e"),
    ("ɐ", "a"),
    ("a", "a"),
    ("ɑ", "a"),
    ("ɒ", "o"),
    ("ɔ", "o"),
    ("o", "o"),
    ("ʊ", "ou"),
    ("u", "ou"),
    ("ʌ", "e"),
    ("ɞ", "eu"),
    ("ɘ", "e"),
    ("ɵ", "eu"),
    ("ɤ", "e"),
    ("ɯ", "i"),
    ("ɨ", "i"),
    ("ʉ", "u"),
    ("ɶ", "eu"),
    ("ɝ", "eur"),
    ("ɚ", "eur"),
    // Consonnes
    ("ɡ", "g"),
    ("ʃ", "ch"),
    ("ʒ", "j"),
    ("ɲ", "gn"),
    ("ŋ", "ng"),
    ("ʁ", "r"),
    ("ʀ", "r"),
    ("ɾ", "r"),
    ("ɹ", "r"),
    ("ɻ", "r"),
    ("ɽ", "r"),
    ("j", "y"),
    ("ɥ", "u"),
    ("w", "ou"),
    ("ʍ", "ou"),
    ("ɦ", "h"),
    ("x", "k"),
    ("ç", "s"),
    ("θ", "s"),
    ("ð", "z"),
    ("ɣ", "g"),
    ("ʝ", "y"),
    ("ɟ", "dy"),
    ("ɢ", "g"),
    ("ʜ", "h"),
    ("ɬ", "hl"),
    ("ɮ", "l"),
    ("ʋ", "v"),
    ("ɱ", "m"),
    ("ɳ", "n"),
    ("ɴ", "n"),
    ("ɰ", "ou"),
    ("ɺ", "r"),
    ("ʙ", "br"),
    ("ⱱ", "v"),
    ("ɓ", "b"),
    ("ɗ", "d"),
    ("ʄ", "dy"),
    ("ɠ", "g"),
    ("ʛ", "g"),
    ("ʔ", ""),
    ("ʕ", ""),
    ("ʢ", ""),
    ("ʡ", ""),
    ("ǀ", ""),
    ("ǁ", ""),
    ("ǂ", ""),
    ("ǃ", ""),
    ("c", "k"),
    ("q", "k"),
    ("p", "p"),
    ("b", "b"),
    ("t", "t"),
    ("d", "d"),
    ("k", "k"),
    ("g", "g"),
    ("f", "f"),
    ("v", "v"),
    ("s", "s"),
    ("z", "z"),
    ("m", "m"),
    ("n", "n"),
    ("l", "l"),
    ("r", "r"),
    ("h", "h"),
];

/// Correspondances API → graphie anglaise.
const EN: &[(&str, &str)] = &[
    // Voyelles nasales (rare en anglais, mais présent dans les emprunts)
    ("ɑ̃", "ahn"),
    ("ɛ̃", "an"),
    ("ɔ̃", "on"),
    ("œ̃", "urn"),
    // Diphtongues
    ("aɪə", "ire"),
    ("aʊə", "ower"),
    ("eɪə", "ayer"),
    ("ɔɪə", "oyer"),
    ("aɪ", "eye"),
    ("aʊ", "ow"),
    ("eɪ", "ay"),
    ("oʊ", "oh"),
    ("əʊ", "oh"),
    ("ɔɪ", "oy"),
    ("ɪə", "ear"),
    ("eə", "air"),
    ("ʊə", "oor"),
    // Affriquées
    ("t͡ʃ", "ch"),
    ("d͡ʒ", "j"),
    ("tʃ", "ch"),
    ("dʒ", "j"),
    ("ʦ", "ts"),
    ("ʣ", "dz"),
    // Voyelles
    ("i", "ee"),
    ("ɪ", "i"),
    ("e", "e"),
    ("ɛ", "e"),
    ("æ", "a"),
    ("a", "ah"),
    ("ɑ", "ah"),
    ("ɒ", "o"),
    ("ɔ", "aw"),
    ("o", "oh"),
    ("ə", "uh"),
    ("ʌ", "u"),
    ("ɜ", "ur"),
    ("ɝ", "ur"),
    ("ɚ", "er"),
    ("u", "oo"),
    ("ʊ", "oo"),
    ("y", "ew"),
    ("ʏ", "ew"),
    ("ø", "ur"),
    ("œ", "ur"),
    ("ɐ", "uh"),
    ("ɞ", "ur"),
    ("ɘ", "uh"),
    ("ɵ", "oh"),
    ("ɤ", "uh"),
    ("ɯ", "oo"),
    ("ɨ", "i"),
    ("ʉ", "oo"),
    ("ɶ", "ur"),
    // Consonnes
    ("ɡ", "g"),
    ("ʃ", "sh"),
    ("ʒ", "zh"),
    ("ɲ", "ny"),
    ("ŋ", "ng"),
    ("ʁ", "r"),
    ("ʀ", "r"),
    ("ɾ", "d"),
    ("ɽ", "r"),
    ("ɹ", "r"),
    ("ɻ", "r"),
    ("j", "y"),
    ("ɥ", "w"),
    ("w", "w"),
    ("ʍ", "w"),
    ("ɦ", "h"),
    ("x", "kh"),
    ("ç", "h"),
    ("θ", "th"),
    ("ð", "dh"),
    ("ɣ", "g"),
    ("ʝ", "y"),
    ("ɟ", "j"),
    ("ɢ", "g"),
    ("ʜ", "h"),
    ("ɬ", "l"),
    ("ɮ", "l"),
    ("ʋ", "v"),
    ("ɱ", "m"),
    ("ɳ", "n"),
    ("ɴ", "n"),
    ("ɰ", "w"),
    ("ɺ", "l"),
    ("ʙ", "br"),
    ("ⱱ", "v"),
    ("ɓ", "b"),
    ("ɗ", "d"),
    ("ʄ", "j"),
    ("ɠ", "g"),
    ("ʛ", "g"),
    ("ʔ", ""),
    ("ʕ", ""),
    ("ʢ", ""),
    ("ʡ", ""),
    ("ǀ", ""),
    ("ǁ", ""),
    ("ǂ", ""),
    ("ǃ", ""),
    ("c", "k"),
    ("q", "k"),
    ("p", "p"),
    ("b", "b"),
    ("t", "t"),
    ("d", "d"),
    ("k", "k"),
    ("g", "g"),
    ("f", "f"),
    ("v", "v"),
    ("s", "s"),
    ("z", "z"),
    ("m", "m"),
    ("n", "n"),
    ("l", "l"),
    ("r", "r"),
    ("h", "h"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecte_une_transcription() {
        assert!(contains_ipa("/ʃa/"));
        assert!(contains_ipa("Le mot [bɔ̃ʒuʁ] est joli"));
        assert!(contains_ipa("həˈloʊ"));
    }

    #[test]
    fn ignore_le_texte_ordinaire() {
        assert!(!contains_ipa("Le chat est petit"));
        assert!(!contains_ipa("Bonjour à tous, ça va ?"));
        assert!(!contains_ipa("Le cœur et l'encyclopædia"));
    }

    #[test]
    fn convertit_un_mot_francais() {
        assert_eq!(to_pronounceable("/ʃa/", "fr"), "cha");
        assert_eq!(to_pronounceable("/bɔ̃ʒuʁ/", "fr"), "bonjour");
        assert_eq!(to_pronounceable("/ɡʁɑ̃d/", "fr"), "grand");
    }

    #[test]
    fn convertit_un_mot_anglais() {
        assert_eq!(to_pronounceable("/ˈwɔːtə/", "en"), "wawtuh");
        assert_eq!(to_pronounceable("[ˈkæt]", "en"), "kat");
        assert_eq!(to_pronounceable("[həˈloʊ]", "en"), "huhloh");
    }

    #[test]
    fn conserve_le_texte_autour_de_la_transcription() {
        assert_eq!(
            to_pronounceable("Le mot « chat » se dit /ʃa/.", "fr"),
            "Le mot « chat » se dit cha."
        );
        assert_eq!(
            to_pronounceable("Le son /ʁ/ est guttural", "fr"),
            "Le son r est guttural"
        );
    }

    #[test]
    fn convertit_une_phrase_entiere() {
        assert_eq!(to_pronounceable("[bɔ̃ʒuʁ lə mɔ̃d]", "fr"), "bonjour le mond");
    }

    #[test]
    fn convertit_les_diacritiques_et_l_accent() {
        assert_eq!(to_pronounceable("/ˈʃat/", "fr"), "chat");
        assert_eq!(to_pronounceable("/t͡ʃa/", "fr"), "tcha");
        assert_eq!(to_pronounceable("/aː/", "fr"), "a");
    }

    #[test]
    fn ne_touche_pas_aux_mots_sans_api() {
        assert_eq!(
            to_pronounceable("Bonjour le monde", "fr"),
            "Bonjour le monde"
        );
    }
}
