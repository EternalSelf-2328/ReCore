//! Soundboard, as Discord's server settings have it: the server's sounds,
//! each renamed or deleted at once, and a new one from a sound file
//! chosen, named and sent. Those with Manage Expressions change every
//! sound, those with Create Expressions only their own.

use super::*;
use crate::chat::{PickedSound, ServerSound, SoundboardAction};

/// How many sounds a server has with `MORE_SOUNDBOARD`.
const MORE_SOUNDBOARD_SLOTS: usize = 96;
/// The slots by boost level without `MORE_SOUNDBOARD`.
const LEVEL_SOUND_SLOTS: [usize; 4] = [8, 24, 36, 48];
pub const SOUND_NAME_LIMIT: usize = 32;
/// Discord takes a sound file of at most 512 KiB.
pub const SOUND_FILE_LIMIT: u64 = 512 * 1024;

/// Something done on Soundboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundboardInput {
    /// The pencil on a sound: its rename dialog.
    Edit(String),
    /// The new sound's name, as typed.
    Name(String),
    Save,
    Cancel,
    Delete(String),
}

/// A sound being added or renamed, as its dialog has it.
#[derive(Clone, Debug, Default)]
pub struct SoundDraft {
    /// The sound being renamed; none for a new one.
    pub editing: Option<String>,
    pub name: String,
    /// A new sound's file, read for it.
    pub file: Option<PickedSound>,
    pub saving: bool,
    pub error: Option<String>,
}

impl SoundDraft {
    /// Whether it may be sent: a name of 2 to 32, and a file for a new
    /// sound.
    pub fn ready(&self) -> bool {
        let name = self.name.trim().chars().count();
        !self.saving
            && (2..=SOUND_NAME_LIMIT).contains(&name)
            && (self.editing.is_some() || self.file.is_some())
    }
}

/// Soundboard, as Discord sent it.
#[derive(Clone, Debug)]
pub struct SoundboardPage {
    pub sounds: Vec<ServerSound>,
    /// How many the server may have, by its boosts.
    pub slots: usize,
    pub draft: Option<SoundDraft>,
}

impl SoundboardPage {
    pub fn new(sounds: Vec<ServerSound>, settings: GuildSettings) -> Self {
        let slots = if settings.has("MORE_SOUNDBOARD") {
            MORE_SOUNDBOARD_SLOTS
        } else {
            LEVEL_SOUND_SLOTS[usize::from(settings.premium_tier.min(3))]
        };
        Self {
            sounds,
            slots,
            draft: None,
        }
    }

    /// How many more sounds fit.
    pub fn free(&self) -> usize {
        self.slots.saturating_sub(self.sounds.len())
    }

    pub fn full(&self) -> bool {
        self.free() == 0
    }

    /// The sounds as Discord sent them again.
    fn answered(&mut self, sounds: Option<Vec<ServerSound>>) {
        if let Some(sounds) = sounds {
            self.sounds = sounds;
        }
    }
}

impl Application {
    pub(in crate::application) fn soundboard_input(
        &mut self,
        input: SoundboardInput,
    ) -> Vec<Effect> {
        let Some(server) = self.settings_server() else {
            return vec![];
        };
        let rules = self.expression_rules(&server);
        let (manage, create, me) = (rules.manage, rules.create, rules.me.map(str::to_owned));
        let rules = ExpressionRules {
            manage,
            create,
            me: me.as_deref(),
        };
        let Some(page) = self.soundboard_page() else {
            return vec![];
        };
        let saving = page.draft.as_ref().is_some_and(|draft| draft.saving);
        let action = match input {
            SoundboardInput::Edit(id) => {
                let sound = page.sounds.iter().find(|sound| sound.id == id);
                let may = sound.is_some_and(|sound| rules.may_change(sound.user_id.as_deref()));
                if let Some(sound) = sound.filter(|_| may && !saving) {
                    page.draft = Some(SoundDraft {
                        editing: Some(id),
                        name: sound.name.clone(),
                        ..SoundDraft::default()
                    });
                }
                return vec![];
            }
            SoundboardInput::Name(name) => {
                if let Some(draft) = page.draft.as_mut().filter(|draft| !draft.saving) {
                    draft.name = name.chars().take(SOUND_NAME_LIMIT).collect();
                }
                return vec![];
            }
            SoundboardInput::Save => {
                let Some(draft) = page.draft.as_mut().filter(|draft| draft.ready()) else {
                    return vec![];
                };
                let name = draft.name.trim().to_owned();
                let action = match (&draft.editing, &draft.file) {
                    (Some(id), _) => {
                        let Some(sound) = page.sounds.iter_mut().find(|sound| sound.id == *id)
                        else {
                            return vec![];
                        };
                        sound.name.clone_from(&name);
                        SoundboardAction::Rename {
                            id: id.clone(),
                            name,
                        }
                    }
                    (None, Some(file)) => {
                        let sound = ServerSound {
                            name: name.clone(),
                            available: true,
                            user_id: me.clone(),
                            uploader: None,
                            ..ServerSound::default()
                        };
                        page.sounds.push(sound);
                        SoundboardAction::Upload {
                            name,
                            sound: file.clone(),
                        }
                    }
                    (None, None) => return vec![],
                };
                if let Some(draft) = page.draft.as_mut() {
                    draft.saving = true;
                    draft.error = None;
                }
                action
            }
            SoundboardInput::Cancel => {
                if !saving {
                    page.draft = None;
                }
                return vec![];
            }
            SoundboardInput::Delete(id) => {
                let owner = page
                    .sounds
                    .iter()
                    .find(|sound| sound.id == id)
                    .map(|sound| sound.user_id.clone());
                if !owner.is_some_and(|owner| rules.may_change(owner.as_deref())) {
                    return vec![];
                }
                // Gone at once, as in Discord's client.
                page.sounds.retain(|sound| sound.id != id);
                SoundboardAction::Delete(id)
            }
        };
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SettingsAction {
                server,
                action: crate::chat::ServerSettingsAction::Soundboard(action),
            },
        ))]
    }

    /// The file chosen for a new sound, read: its dialog opens with it.
    pub(in crate::application) fn server_sound_read(
        &mut self,
        path: std::path::PathBuf,
        result: Result<PickedSound, String>,
    ) {
        let Some(page) = self.soundboard_page() else {
            return;
        };
        if page.draft.as_ref().is_some_and(|draft| draft.saving) {
            return;
        }
        match result {
            Ok(sound) => {
                let mut name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                name.truncate(SOUND_NAME_LIMIT);
                page.draft = Some(SoundDraft {
                    name,
                    file: Some(sound),
                    ..SoundDraft::default()
                });
            }
            Err(error) => {
                if let Some(settings) = &mut self.state.server_settings {
                    settings.error = Some(error);
                }
            }
        }
    }

    /// What Soundboard asked of Discord, and the sounds after it.
    pub(in crate::application) fn soundboard_changed(
        &mut self,
        server: String,
        action: SoundboardAction,
        result: Result<(), String>,
        sounds: Option<Vec<ServerSound>>,
    ) {
        if self.settings_server().as_deref() != Some(&server) {
            return;
        }
        let mut error = None;
        let Some(page) = self.soundboard_page() else {
            return;
        };
        page.answered(sounds);
        match (action, result) {
            (SoundboardAction::Delete(_), result) => error = result.err(),
            (_, Ok(())) => page.draft = None,
            (_, Err(why)) => {
                if let Some(draft) = &mut page.draft {
                    draft.saving = false;
                    draft.error = Some(why);
                }
            }
        }
        if let Some(settings) = &mut self.state.server_settings {
            settings.error = error;
        }
    }

    fn soundboard_page(&mut self) -> Option<&mut SoundboardPage> {
        match self.state.server_settings.as_mut()?.page.as_mut()? {
            Page::Soundboard(page) => Some(page),
            _ => None,
        }
    }
}
