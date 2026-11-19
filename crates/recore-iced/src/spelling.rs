//! Words typed without their accents: while a word is being written, one
//! that is only right with an accent is offered under the composer. Nothing
//! is replaced on its own: Tab, or a press on the offer, takes it, and the
//! suggestion goes as the word changes. Messages' switch turns it off.
//!
//! The list holds words whose unaccented spelling is another word, or no
//! word at all, in Spanish: `pasion`, `telefono`, `tambien`. Words like
//! `esta`/`está`, `el`/`él` or `si`/`sí` are left out, since which one is
//! meant is the sentence's.

use std::{cmp::Ordering, sync::OnceLock};

use crate::{app::App, composer::Writing};

/// The words offered when their accents are missing.
const WORDS: &[&str] = &[
    "acción",
    "aplicación",
    "aceptación",
    "además",
    "actualización",
    "alegría",
    "álbum",
    "allí",
    "aquí",
    "árbol",
    "armonía",
    "así",
    "atención",
    "avión",
    "básico",
    "batería",
    "bebé",
    "biografía",
    "biología",
    "botón",
    "café",
    "caída",
    "caligrafía",
    "camión",
    "canción",
    "categoría",
    "colección",
    "comió",
    "comunicación",
    "compañía",
    "condición",
    "configuración",
    "conexión",
    "corazón",
    "creación",
    "crédito",
    "déficit",
    "decisión",
    "decoración",
    "después",
    "día",
    "difícil",
    "dirección",
    "discusión",
    "economía",
    "educación",
    "elección",
    "emoción",
    "energía",
    "estación",
    "fácil",
    "filosofía",
    "física",
    "físico",
    "fotografía",
    "fracción",
    "francés",
    "función",
    "fútbol",
    "galería",
    "garantía",
    "generación",
    "geografía",
    "guión",
    "habrá",
    "hará",
    "hábitat",
    "información",
    "inglés",
    "instalación",
    "intención",
    "ironía",
    "jamón",
    "jardín",
    "joyería",
    "lápiz",
    "lección",
    "librería",
    "limón",
    "lógica",
    "lógico",
    "mágico",
    "maíz",
    "mamá",
    "mayoría",
    "melón",
    "menú",
    "método",
    "micrófono",
    "minoría",
    "misión",
    "monotonía",
    "música",
    "nación",
    "ocasión",
    "oído",
    "opción",
    "oír",
    "oirá",
    "operación",
    "organización",
    "ortografía",
    "panadería",
    "papá",
    "papelería",
    "país",
    "pasión",
    "pastelería",
    "patín",
    "peluquería",
    "perfumería",
    "policía",
    "posición",
    "podrá",
    "pondrá",
    "próximo",
    "publicación",
    "querrá",
    "rápido",
    "raíz",
    "raíces",
    "ratón",
    "razón",
    "reacción",
    "récord",
    "región",
    "relación",
    "religión",
    "revelación",
    "sabrá",
    "saldrá",
    "sandía",
    "satisfacción",
    "según",
    "será",
    "sesión",
    "sinfonía",
    "sofá",
    "solución",
    "sastrería",
    "también",
    "técnica",
    "técnico",
    "tecnología",
    "teléfono",
    "televisión",
    "tendrá",
    "tía",
    "tío",
    "todavía",
    "tradición",
    "tráfico",
    "versión",
    "verá",
    "violín",
    "vivió",
    "zapatería",
];

/// A word under the composer's caret, its letters and the word it should be.
pub struct Suggestion {
    /// Where the word starts, in the draft's bytes.
    at: usize,
    /// The letters written.
    word: String,
    /// The word they should be.
    fixed: String,
}

impl Suggestion {
    /// The letters written.
    pub fn word(&self) -> &str {
        &self.word
    }

    /// The word offered.
    pub fn fixed(&self) -> &str {
        &self.fixed
    }
}

/// The word being written in the draft, `text`, with the caret at `caret`,
/// when its accents are missing and the list knows it.
pub fn for_draft(text: &str, caret: usize) -> Option<Suggestion> {
    let mut caret = caret.min(text.len());
    while caret > 0 && !text.is_char_boundary(caret) {
        caret -= 1;
    }
    let before = &text[..caret];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_alphabetic())
        .map(|(at, c)| at + c.len_utf8())
        .unwrap_or(0);
    let word = &before[start..];
    let letters = word.chars().count();
    if !(3..=24).contains(&letters) || !word.chars().all(char::is_alphabetic) {
        return None;
    }
    // A mention, a channel, an emoji's name or a command isn't a word to
    // correct.
    if start > 0
        && matches!(
            before[..start].chars().next_back(),
            Some('@' | '#' | ':' | '/')
        )
    {
        return None;
    }
    // A word already carrying an accent is taken as written.
    if !word.is_ascii() {
        return None;
    }
    let fixed = lookup(&word)?;
    Some(Suggestion {
        at: start,
        word: word.to_owned(),
        fixed: case_like(word, fixed),
    })
}

/// The word in the list whose letters are `typed`'s, found by comparing the
/// typed word against a table of the list's folded spellings: a binary
/// search, and no allocation for a word typed.
fn lookup(typed: &str) -> Option<&'static str> {
    let table = folded_table();
    let at = table
        .binary_search_by(|(folded, _)| compare(folded, typed))
        .ok()?;
    Some(table[at].1)
}

/// How `typed` orders against a folded spelling, by bytes with the typed
/// letters lowercased: the typed word is ASCII, and so is every folded one.
fn compare(folded: &str, typed: &str) -> Ordering {
    let (folded, typed) = (folded.as_bytes(), typed.as_bytes());
    for (a, b) in folded.iter().zip(typed) {
        match a.cmp(&b.to_ascii_lowercase()) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    folded.len().cmp(&typed.len())
}

/// The list once, each word with its letters folded and no accents, sorted
/// by them for [`lookup`]: about 160 words, made on the first word typed.
fn folded_table() -> &'static [(String, &'static str)] {
    static TABLE: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: Vec<_> = WORDS.iter().map(|word| (folded(word), *word)).collect();
        table.sort_unstable_by(|(this, _), (that, _)| this.cmp(that));
        table
    })
}

/// An input method's preedit changed: the word it holds, which isn't in the
/// core's draft yet, is offered all the same, so Tab can take it before the
/// method commits it.
pub fn composing(app: &mut App, which: Writing) {
    if which != Writing::Draft {
        return;
    }
    let offer = app.layout.prefs.word_suggestions.unwrap_or(true);
    app.composer.spelling = offer
        .then(|| {
            let draft = &app.composer.draft;
            let text = draft.text();
            for_draft(&text, draft.caret())
        })
        .flatten();
}

/// The suggestion still matching the draft and its caret, for the bar and
/// the field under them.
pub fn current(composer: &crate::composer::Composer) -> Option<&Suggestion> {
    let suggestion = composer.spelling.as_ref()?;
    let caret = composer.draft.caret();
    (caret == suggestion.at + suggestion.word.len()
        && composer.draft.text_at(suggestion.at, suggestion.word()))
    .then_some(suggestion)
}

/// Takes the suggestion under the composer, if it still matches the draft.
pub fn apply(app: &mut App) -> bool {
    let Some(suggestion) = app.composer.spelling.take() else {
        return false;
    };
    if for_draft(&app.composer.draft.text(), app.composer.draft.caret()).is_none() {
        return false;
    }
    let text = app.composer.draft.text();
    let mut caret = app.composer.draft.caret().min(text.len());
    while caret > 0 && !text.is_char_boundary(caret) {
        caret -= 1;
    }
    if text.get(suggestion.at..caret) != Some(suggestion.word.as_str()) {
        return false;
    }
    let mut fixed = String::with_capacity(text.len() + 4);
    fixed.push_str(&text[..suggestion.at]);
    fixed.push_str(&suggestion.fixed);
    fixed.push_str(&text[caret..]);
    let at = suggestion.at + suggestion.fixed.len();
    crate::composer::replace_text(app, Writing::Draft, fixed, at);
    true
}

/// `known` with `word`'s capital: all caps stay all caps, a first capital
/// stays first, and the rest is the list's.
fn case_like(word: &str, known: &str) -> String {
    let upper = word.chars().filter(|c| c.is_uppercase()).count();
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    if letters > 1 && upper == letters {
        return known.to_uppercase();
    }
    if word.chars().next().is_some_and(char::is_uppercase) {
        let mut chars = known.chars();
        if let Some(first) = chars.next() {
            return first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    known.to_owned()
}

/// `text` lowercased with its accents taken off, to compare with what was
/// typed.
fn folded(text: &str) -> String {
    text.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_missing_its_accent_is_offered() {
        let suggestion = for_draft("hola pasion", 11).expect("a suggestion");
        assert_eq!(suggestion.word(), "pasion");
        assert_eq!(suggestion.fixed(), "pasión");
        assert_eq!(
            for_draft("Hola Pasion", 11).expect("a suggestion").fixed(),
            "Pasión"
        );
        assert_eq!(
            for_draft("HOLA PASION", 11).expect("a suggestion").fixed(),
            "PASIÓN"
        );
    }

    #[test]
    fn words_that_are_right_unaccented_or_already_accented_are_left() {
        assert!(for_draft("esta", 4).is_none(), "a word of its own");
        assert!(for_draft("está", 4).is_none(), "already accented");
        assert!(for_draft("pasión", 6).is_none());
        assert!(for_draft("p", 1).is_none(), "too short");
        assert!(for_draft("@pasion", 7).is_none(), "a mention");
        assert!(for_draft(":pasion", 7).is_none(), "an emoji's name");
        assert!(for_draft("pasion ", 7).is_none(), "the word ended");
    }

    #[test]
    fn the_caret_in_the_middle_still_finds_the_word() {
        let suggestion = for_draft("una cancion triste", 11).expect("a suggestion");
        assert_eq!(suggestion.at, 4);
        assert_eq!(suggestion.fixed(), "canción");
    }

    #[test]
    fn every_word_is_found_by_its_letters_without_their_accents() {
        for word in WORDS {
            assert_eq!(lookup(&folded(word)), Some(*word), "{word}");
        }
    }

    #[test]
    fn the_folded_spellings_are_sorted_and_distinct() {
        let table = folded_table();
        assert!(
            table.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "sorted, each spelling once"
        );
    }

    #[test]
    fn the_lookup_ignores_case_without_allocating_a_lowercase_word() {
        assert_eq!(lookup("PASION"), Some("pasión"));
        assert_eq!(lookup("Pasion"), Some("pasión"));
        assert_eq!(lookup("teleFONO"), Some("teléfono"));
        assert_eq!(lookup("pasioncita"), None);
        assert_eq!(lookup("pasio"), None);
    }
}
