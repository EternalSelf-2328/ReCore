//! Autocorrect's language model: a word-frequency list downloaded for the
//! language the account writes in, and the suggestions made from it.
//!
//! The model is the FrequencyWords list for the language, a line of the
//! word and how often it was written in subtitles, and is only fetched when
//! the person asks for it. A word missing its accents, or a word that is no
//! word at all but one edit away from a common one, is offered under the
//! composer. Nothing changes on its own: Tab, or a press, takes the offer.
//! Without a model, the Spanish accents in `spelling.rs` still work.

use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use iced::Task;

use crate::{
    app::{App, Message},
    composer::Writing,
    spelling::{self, Suggestion},
};

/// The model's files live here under the data folder.
const FOLDER: &str = "autocorrect";
/// The most a model file may take; the lists are about 600 KiB.
const MOST_BYTES: usize = 8 * 1024 * 1024;
/// How long a download may take before it is given up.
const WAIT: Duration = Duration::from_secs(45);
/// Where the word lists come from. Their content is CC BY-SA 4.0.
const SOURCE: &str = concat!(
    "https://raw.githubusercontent.com/hermitdave/FrequencyWords/",
    "525f9b560de45753a5ea01069454e72e9aa541c6/content/2018"
);
// The commit is pinned: the lists' words and their licence stay as checked
// here, whatever changes on master.
const NOTICE: &str = "\
Word frequencies from FrequencyWords by Hermit Dave
https://github.com/hermitdave/FrequencyWords
Built from OpenSubtitles2018. The lists are licensed CC BY-SA 4.0:
https://creativecommons.org/licenses/by-sa/4.0/
";

/// A language the frequency lists have: its code, its list's folder in the
/// source, and its name for the interface.
const LANGUAGES: &[(&str, &str, &str, &str)] = &[
    ("en", "en", "en_50k.txt", "English"),
    ("es", "es", "es_50k.txt", "Spanish"),
    ("pt", "pt", "pt_50k.txt", "Portuguese"),
    ("fr", "fr", "fr_50k.txt", "French"),
    ("de", "de", "de_50k.txt", "German"),
    ("it", "it", "it_50k.txt", "Italian"),
    ("nl", "nl", "nl_50k.txt", "Dutch"),
    // The Japanese list has no 50k cut.
    ("ja", "ja", "ja_full.txt", "Japanese"),
    ("ko", "ko", "ko_50k.txt", "Korean"),
    ("zh-CN", "zh_cn", "zh_cn_50k.txt", "Chinese (Simplified)"),
    ("ru", "ru", "ru_50k.txt", "Russian"),
    ("ar", "ar", "ar_50k.txt", "Arabic"),
];

/// A downloaded model, kept for one language.
#[derive(Debug)]
pub struct Model {
    /// Every word, sorted by its folded spelling and then by how often it
    /// is written, so the first of a folded spelling is the one offered.
    words: Vec<Word>,
    /// The words' places, sorted by their spelling as written, for telling
    /// whether a word is one the language knows.
    by_text: Vec<u32>,
    /// The letters its words are made of, commonest first, for the typos
    /// made by a nearby key. A script with too many of them (Chinese,
    /// Japanese) gets no typo suggestions.
    letters: Vec<char>,
    /// How many letters the whole list uses, before the commonest were
    /// kept for [`Model::letters`].
    distinct: usize,
}

#[derive(Debug)]
struct Word {
    /// The word as the list writes it, lowercased.
    text: Box<str>,
    /// The word with its accents taken off, the form typed on a keyboard
    /// without them.
    folded: Box<str>,
    count: u64,
}

/// What the model is doing for the chosen language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// No model is downloaded; Spanish accents are all there is.
    Missing,
    Downloading,
    Ready {
        words: usize,
    },
    Failed(String),
}

/// The model and its download, for the language written in.
pub struct Autocorrect {
    /// The language code, as `translate::LANGUAGES` names it.
    pub language: String,
    pub model: Option<Arc<Model>>,
    pub status: Status,
}

/// What the model's settings and download offer.
#[derive(Clone)]
pub enum Event {
    Download,
    Remove,
    /// A download finished for `language`, whether it's still the chosen
    /// one or another was picked meanwhile.
    Downloaded(String, Result<Arc<Model>, String>),
}

impl Autocorrect {
    /// The model kept for `language`, or the state of its download.
    pub fn load(root: &Path, language: String) -> Self {
        let path = model_path(root, &language);
        let (model, status) = match fs::read_to_string(&path) {
            Ok(text) => match Model::parse(&language, &text) {
                Ok(model) => {
                    let words = model.words.len();
                    (Some(Arc::new(model)), Status::Ready { words })
                }
                Err(error) => (None, Status::Failed(error)),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, Status::Missing),
            Err(error) => (
                None,
                Status::Failed(format!("Could not read the model: {error}")),
            ),
        };
        Self {
            language,
            model,
            status,
        }
    }

    /// Takes the model for `language`, which Settings just changed, or the
    /// one this device kept for it.
    pub fn reload(&mut self, root: &Path, language: String) {
        *self = Self::load(root, language);
    }

    /// The name of the language for a message, as Settings names it.
    pub fn language_name(&self) -> String {
        LANGUAGES
            .iter()
            .find(|(code, ..)| code.eq_ignore_ascii_case(&self.language))
            .map_or_else(
                || crate::translate::language_name(&self.language),
                |(_, _, _, name)| (*name).to_owned(),
            )
    }
}

/// The language autocorrect uses: the one Messages says messages are written
/// in, or else the desktop's language, and English when neither is one the
/// lists have.
pub fn language(prefs: &crate::layout::Prefs) -> String {
    prefs
        .translate_source
        .as_deref()
        .and_then(normalize)
        .or_else(system_language)
        .unwrap_or_else(|| "en".to_owned())
}

/// `code` as one of [`LANGUAGES`]' codes, if the lists have it.
fn normalize(code: &str) -> Option<String> {
    let code = code.to_ascii_lowercase().replace('_', "-");
    let code = code.split(['.', '@']).next().unwrap_or(&code);
    let base = code.split('-').next().unwrap_or(code);
    let code = match base {
        "zh" => "zh-CN",
        _ => base,
    };
    LANGUAGES
        .iter()
        .find(|(known, ..)| known.eq_ignore_ascii_case(code))
        .map(|(known, ..)| (*known).to_owned())
}

/// The desktop's language, from the variables a Linux session sets.
fn system_language() -> Option<String> {
    for variable in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        let Ok(value) = env::var(variable) else {
            continue;
        };
        for part in value.split(':') {
            if let Some(code) = normalize(part) {
                return Some(code);
            }
        }
    }
    None
}

/// The model's file in the data folder.
fn model_path(root: &Path, language: &str) -> PathBuf {
    root.join(FOLDER).join(format!("{language}.txt"))
}

/// The folder and file of `language`'s word list.
fn source(language: &str) -> Option<(&'static str, &'static str)> {
    LANGUAGES
        .iter()
        .find(|(code, ..)| code.eq_ignore_ascii_case(language))
        .map(|(_, folder, file, _)| (*folder, *file))
}

impl Model {
    /// The list as `language`'s model: `word count` a line, as
    /// FrequencyWords writes it.
    fn parse(_language: &str, text: &str) -> Result<Self, String> {
        let mut words = Vec::with_capacity(50_000);
        let mut used = HashMap::new();
        for line in text.lines() {
            let Some((word, count)) = line.trim_end().rsplit_once(' ') else {
                continue;
            };
            let Ok(count) = count.parse::<u64>() else {
                continue;
            };
            let letters = word.chars().count();
            if count == 0 || !(1..=32).contains(&letters) {
                continue;
            }
            if !word.chars().all(char::is_alphabetic) {
                continue;
            }
            if words.len() >= 100_000 {
                return Err("The downloaded file has too many words to be one list.".to_owned());
            }
            let lower = word.to_lowercase();
            let folded = fold(&lower);
            for letter in folded.chars() {
                *used.entry(letter).or_insert(0u32) += 1;
            }
            words.push(Word {
                folded: folded.into_boxed_str(),
                text: lower.into_boxed_str(),
                count,
            });
        }
        if words.len() < 5 {
            return Err("The downloaded file isn't a word list.".to_owned());
        }
        // The most written word of a folded spelling comes first, so it's
        // the one offered; the order is otherwise stable for the search.
        words.sort_unstable_by(|this, that| {
            this.folded
                .cmp(&that.folded)
                .then_with(|| that.count.cmp(&this.count))
        });
        let mut by_text = (0..words.len() as u32).collect::<Vec<_>>();
        by_text.sort_unstable_by(|&this, &that| {
            words[this as usize]
                .text
                .cmp(&words[that as usize].text)
                .then_with(|| words[that as usize].count.cmp(&words[this as usize].count))
        });
        let distinct = used.len();
        let mut letters = used.into_iter().collect::<Vec<_>>();
        letters.sort_unstable_by(|(_, this), (_, that)| that.cmp(this));
        let letters = letters
            .into_iter()
            .take(64)
            .map(|(letter, _)| letter)
            .collect();
        Ok(Self {
            words,
            by_text,
            letters,
            distinct,
        })
    }

    /// The most written word whose letters are `folded`'s.
    fn folded_at(&self, folded: &str) -> Option<&Word> {
        let at = self
            .words
            .partition_point(|word| word.folded.as_ref() < folded);
        let word = self.words.get(at)?;
        (word.folded.as_ref() == folded).then_some(word)
    }

    /// The word written `text`, if the language has it.
    fn exact_at(&self, text: &str) -> Option<&Word> {
        let at = self
            .by_text
            .binary_search_by(|&index| self.words[index as usize].text.as_ref().cmp(text));
        at.ok().map(|at| &self.words[self.by_text[at] as usize])
    }

    /// The word offered for `typed`: the same word with its accents when
    /// they're missing, or, when `corrections`, the common word one edit
    /// away. A word the language knows is never changed.
    fn suggest(&self, typed: &str, corrections: bool) -> Option<String> {
        let lower = typed.to_lowercase();
        let folded = fold(&lower);
        if let Some(best) = self.folded_at(&folded)
            && best.text.as_ref() != lower
        {
            // An unaccented spelling that is a word of its own is left to
            // the sentence, as `spelling.rs` leaves esta/está. A rare one,
            // such as pasion, is offered its accents.
            let common = self
                .exact_at(&lower)
                .is_some_and(|typed| typed.count.saturating_mul(20) >= best.count);
            if !common {
                return Some(best.text.to_string());
            }
        }
        let exact = self.exact_at(&lower);
        // A word the list knows is taken as typed, unless it is so rare
        // that the commonest word one edit away is plainly what was meant.
        if exact.is_some_and(|word| word.count >= 1_000) {
            return None;
        }
        if corrections && self.distinct <= 96 && (3..=24).contains(&lower.chars().count()) {
            let found = self.nearest(&lower)?;
            if exact.is_none_or(|typed| found.count > typed.count.saturating_mul(20)) {
                return Some(found.text.to_string());
            }
        }
        None
    }

    /// The most written word one edit from `word`, if there is one.
    fn nearest(&self, word: &str) -> Option<&Word> {
        let chars: Vec<char> = word.chars().collect();
        let mut best: Option<&Word> = None;
        let mut consider = |candidate: String| {
            if candidate == word {
                return;
            }
            let Some(found) = self.folded_at(&fold(&candidate)) else {
                return;
            };
            if best.is_none_or(|best| {
                found.count > best.count
                    || (found.count == best.count && found.text.len() < best.text.len())
            }) {
                best = Some(found);
            }
        };
        for at in 0..chars.len() {
            let mut deletion = chars.clone();
            deletion.remove(at);
            consider(deletion.into_iter().collect());
            let mut replacement = chars.clone();
            for &letter in &self.letters {
                replacement[at] = letter;
                consider(replacement.iter().collect());
            }
            if at + 1 < chars.len() {
                let mut transposed = chars.clone();
                transposed.swap(at, at + 1);
                consider(transposed.into_iter().collect());
            }
        }
        for at in 0..=chars.len() {
            for &letter in &self.letters {
                let mut insertion = chars.clone();
                insertion.insert(at, letter);
                consider(insertion.into_iter().collect());
            }
        }
        best
    }
}

/// The word offered under the composer, if the model or the built-in
/// Spanish list knows one: the word at the caret with its accents, or else
/// the word just finished with a typo.
pub fn for_draft(app: &App, text: &str, caret: usize) -> Option<Suggestion> {
    let language = app.autocorrect.language.as_str();
    let model = app.autocorrect.model.as_deref();
    if let Some(model) = model
        && let Some((at, word)) = spelling::word_before(text, caret)
        && let Some(fixed) = model.suggest(word, false)
    {
        return Some(Suggestion::new(at, word, spelling::case_like(word, &fixed)));
    }
    if let Some(suggestion) = spelling::for_language(text, caret, language) {
        return Some(suggestion);
    }
    if let Some(model) = model
        && let Some((at, word)) = spelling::finished_word(text, caret)
        && let Some(fixed) = model.suggest(word, true)
    {
        return Some(Suggestion::new(at, word, spelling::case_like(word, &fixed)));
    }
    None
}

/// An input method's preedit changed: the word it holds is offered too.
pub fn composing(app: &mut App, which: Writing) {
    if which != Writing::Draft {
        return;
    }
    let offer = app.layout.prefs.word_suggestions.unwrap_or(true);
    app.composer.spelling = offer
        .then(|| {
            let draft = &app.composer.draft;
            let text = draft.text();
            for_draft(app, &text, draft.caret())
        })
        .flatten();
}

/// A download, removal or finished download from Settings.
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Download => download(app),
        Event::Remove => {
            drop(fs::remove_file(model_path(
                app.client.root(),
                &app.autocorrect.language,
            )));
            app.autocorrect.model = None;
            app.autocorrect.status = Status::Missing;
            app.composer.spelling = None;
            Task::none()
        }
        Event::Downloaded(language, result) => {
            if language == app.autocorrect.language {
                match result {
                    Ok(model) => {
                        let words = model.words.len();
                        app.autocorrect.model = Some(model);
                        app.autocorrect.status = Status::Ready { words };
                    }
                    Err(error) => app.autocorrect.status = Status::Failed(error),
                }
                app.composer.spelling = None;
            }
            Task::none()
        }
    }
}

/// Asks for the chosen language's list and keeps it beside the layout.
fn download(app: &mut App) -> Task<Message> {
    if app.autocorrect.model.is_some() || matches!(app.autocorrect.status, Status::Downloading) {
        return Task::none();
    }
    let language = app.autocorrect.language.clone();
    let Some((folder, file)) = source(&language) else {
        app.autocorrect.status = Status::Failed("The lists have no such language.".to_owned());
        return Task::none();
    };
    let url = format!("{SOURCE}/{folder}/{file}");
    let root = app.client.root().to_owned();
    app.autocorrect.status = Status::Downloading;
    let told = language.clone();
    Task::perform(fetch(language, url, root), move |result| {
        Message::Autocorrect(Event::Downloaded(told, result))
    })
}

/// Downloads, checks and saves one language's list, then parses it off the
/// window's thread.
async fn fetch(language: String, url: String, root: PathBuf) -> Result<Arc<Model>, String> {
    // The window starts the provider; a test or a library caller may not.
    drop(recore_core::init_tls());
    let client = reqwest::Client::builder()
        .user_agent("ReCore autocorrect model downloader")
        .timeout(WAIT)
        .build()
        .map_err(|error| format!("Could not start the download: {error}"))?;
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Could not reach the model's address: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "The model's address answered {}.",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MOST_BYTES as u64)
    {
        return Err("The downloaded file is too large to be a word list.".to_owned());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Could not read the model: {error}"))?
    {
        if bytes.len() + chunk.len() > MOST_BYTES {
            return Err("The downloaded file is too large to be a word list.".to_owned());
        }
        bytes.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || {
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| "The downloaded file isn't text.".to_owned())?;
        let model = Model::parse(&language, text)?;
        let folder = root.join(FOLDER);
        fs::create_dir_all(&folder)
            .map_err(|error| format!("Could not keep the model: {error}"))?;
        // Written aside and renamed, so a start meets a whole list or the
        // one before it, never half a download.
        let file = folder.join(format!("{language}.txt"));
        let next = folder.join(format!("{language}.part"));
        fs::write(&next, text).map_err(|error| format!("Could not keep the model: {error}"))?;
        fs::rename(&next, &file).map_err(|error| format!("Could not keep the model: {error}"))?;
        fs::write(folder.join("NOTICE.txt"), NOTICE)
            .map_err(|error| format!("Could not keep the model's notice: {error}"))?;
        Ok(Arc::new(model))
    })
    .await
    .map_err(|_| "The model's reader stopped before it finished.".to_owned())?
}

/// A word without its accents, so a keyboard without them still finds it.
fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => folded.push('a'),
            'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => folded.push('e'),
            'í' | 'ì' | 'î' | 'ï' | 'ī' | 'ĭ' | 'į' | 'ı' => folded.push('i'),
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' | 'ŏ' | 'ő' => folded.push('o'),
            'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => folded.push('u'),
            'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => folded.push('c'),
            'ñ' | 'ń' | 'ņ' | 'ň' => folded.push('n'),
            'ý' | 'ÿ' | 'ŷ' => folded.push('y'),
            'ś' | 'š' | 'ş' | 'ș' => folded.push('s'),
            'ź' | 'ż' | 'ž' => folded.push('z'),
            'ğ' | 'ģ' | 'ġ' => folded.push('g'),
            'ř' | 'ŕ' => folded.push('r'),
            'ť' | 'ţ' | 'ț' => folded.push('t'),
            'ď' | 'đ' | 'ð' => folded.push('d'),
            'ĺ' | 'ľ' | 'ļ' | 'ł' => folded.push('l'),
            'ß' => folded.push_str("ss"),
            'æ' | 'ǽ' => folded.push_str("ae"),
            'œ' | 'Œ' => folded.push_str("oe"),
            'þ' => folded.push_str("th"),
            other => folded.extend(other.to_lowercase()),
        }
    }
    folded
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPANISH: &str = "\
que 100000
de 90000
la 80000
el 70000
esta 10519
está 24332
pasión 10592
pasion 237
cancion 120
canción 25000
telefono 3246
teléfono 95015
";

    const ENGLISH: &str = "\
the 200000
and 150000
receive 9000
form 7000
from 6000
que 5000000
qeu 195
";

    fn model(language: &str, text: &str) -> Model {
        Model::parse(language, text).expect("a model")
    }

    #[test]
    fn a_word_without_its_accents_is_offered_its_own() {
        let model = model("es", SPANISH);
        assert_eq!(model.suggest("pasion", false).as_deref(), Some("pasión"));
        assert_eq!(
            model.suggest("telefono", false).as_deref(),
            Some("teléfono")
        );
        assert_eq!(model.suggest("cancion", false).as_deref(), Some("canción"));
    }

    #[test]
    fn a_word_that_is_a_word_without_accents_is_left_alone() {
        let model = model("es", SPANISH);
        assert_eq!(model.suggest("esta", false), None);
    }

    #[test]
    fn a_common_word_one_edit_away_is_corrected_when_asked() {
        let model = model("en", ENGLISH);
        assert_eq!(model.suggest("teh", true).as_deref(), Some("the"));
        assert_eq!(model.suggest("recieve", true).as_deref(), Some("receive"));
        assert_eq!(model.suggest("recive", true).as_deref(), Some("receive"));
        assert_eq!(model.suggest("recieve", false), None);
        // A typo the list has seen once or twice is still a typo.
        assert_eq!(model.suggest("qeu", true).as_deref(), Some("que"));
    }

    #[test]
    fn a_word_the_language_knows_is_not_changed() {
        let model = model("en", ENGLISH);
        assert_eq!(model.suggest("form", true), None);
        assert_eq!(model.suggest("from", true), None);
    }

    #[test]
    fn the_language_follows_the_desktop_when_none_is_chosen() {
        assert_eq!(normalize("es_ES.UTF-8").as_deref(), Some("es"));
        assert_eq!(normalize("zh_CN.UTF-8").as_deref(), Some("zh-CN"));
        assert_eq!(normalize("pt_BR.UTF-8").as_deref(), Some("pt"));
    }
}
