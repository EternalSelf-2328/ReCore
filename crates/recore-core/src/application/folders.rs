//! Server folders: making one, changing its name, colour and servers, and
//! deleting it, through Discord's settings protobuf.

use super::*;

impl Application {
    /// Opens the folder editor on a new folder holding `server`.
    pub(super) fn new_folder(&mut self, server: String) -> Vec<Effect> {
        if self.state.preview || !server.starts_with("discord:") {
            return vec![];
        }
        let Some(found) = self.state.servers.iter().find(|one| one.id == server) else {
            return vec![];
        };
        let mut editor = self.folder_editor(None);
        for one in &mut editor.servers {
            one.chosen = one.id == found.id;
        }
        self.state.folder_editor = Some(editor);
        vec![]
    }

    /// Opens the folder editor on an existing folder, by its rail id.
    pub(super) fn edit_folder(&mut self, id: String) -> Vec<Effect> {
        if self.state.preview {
            return vec![];
        }
        let Some(folder) = self
            .state
            .servers
            .iter()
            .find_map(|server| server.folder.as_ref().filter(|folder| folder.id == id))
        else {
            return vec![];
        };
        let mut editor = self.folder_editor(Some(folder.id.clone()));
        editor.name = folder.name.clone().unwrap_or_default();
        editor.color = folder.color;
        for one in &mut editor.servers {
            one.chosen = self
                .state
                .servers
                .iter()
                .find(|server| server.id == one.id)
                .and_then(|server| server.folder.as_ref())
                .is_some_and(|folder| folder.id == id);
        }
        self.state.folder_editor = Some(editor);
        vec![]
    }

    fn folder_editor(&self, id: Option<String>) -> FolderEditor {
        FolderEditor {
            id,
            servers: self
                .state
                .servers
                .iter()
                .filter(|server| server.discord)
                .map(|server| FolderServer {
                    id: server.id.clone(),
                    name: server.name.clone(),
                    icon: server.icon.clone(),
                    chosen: false,
                })
                .collect(),
            ..Default::default()
        }
    }

    pub(super) fn set_folder_name(&mut self, name: String) -> Vec<Effect> {
        if let Some(editor) = &mut self.state.folder_editor {
            editor.name = name;
            editor.error = None;
        }
        vec![]
    }

    pub(super) fn set_folder_color(&mut self, color: Option<u32>) -> Vec<Effect> {
        if let Some(editor) = &mut self.state.folder_editor {
            editor.color = color;
        }
        vec![]
    }

    pub(super) fn toggle_folder_server(&mut self, server: String) -> Vec<Effect> {
        if let Some(editor) = &mut self.state.folder_editor
            && let Some(one) = editor.servers.iter_mut().find(|one| one.id == server)
        {
            one.chosen = !one.chosen;
            editor.error = None;
        }
        vec![]
    }

    pub(super) fn save_folder(&mut self) -> Vec<Effect> {
        let Some(editor) = self.state.folder_editor.as_ref() else {
            return vec![];
        };
        if editor.saving {
            return vec![];
        }
        if !editor.servers.iter().any(|server| server.chosen) {
            if let Some(editor) = &mut self.state.folder_editor {
                editor.error = Some("Choose at least one server for the folder.".into());
            }
            return vec![];
        }
        let target = editor_target(editor, &self.state.servers);
        let folders = self.folders_with(Some((target, Some(editor))));
        if let Some(editor) = &mut self.state.folder_editor {
            editor.saving = true;
            editor.error = None;
        }
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SetServerFolders(folders),
        ))]
    }

    pub(super) fn delete_folder(&mut self) -> Vec<Effect> {
        let Some(editor) = &self.state.folder_editor else {
            return vec![];
        };
        let Some(id) = editor.id.clone() else {
            self.state.folder_editor = None;
            return vec![];
        };
        if editor.saving {
            return vec![];
        }
        let folders = self.folders_with(Some((id, None)));
        if let Some(editor) = &mut self.state.folder_editor {
            editor.saving = true;
            editor.error = None;
        }
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SetServerFolders(folders),
        ))]
    }

    /// The server list's settings with `change` applied: a folder's id and
    /// its new contents, or the same with none to delete it.
    fn folders_with(
        &self,
        change: Option<(String, Option<&FolderEditor>)>,
    ) -> Vec<recore_discord::settings::Folder> {
        use recore_discord::settings::Folder;
        let (target, editor) = match change {
            Some((id, editor)) => (Some(id), editor),
            None => (None, None),
        };
        let mut entries: Vec<Folder> = Vec::new();
        let mut placed_target = false;
        for server in &self.state.servers {
            let Some(guild) = discord::parse_server(&server.id) else {
                continue;
            };
            let chosen = editor.is_some_and(|editor| {
                editor
                    .servers
                    .iter()
                    .any(|one| one.id == server.id && one.chosen)
            });
            if chosen && let Some(target) = &target {
                if !placed_target {
                    let existing = self.state.servers.iter().find_map(|server| {
                        server.folder.as_ref().filter(|folder| &folder.id == target)
                    });
                    entries.push(Folder {
                        id: target.trim_start_matches("discord-folder:").to_owned(),
                        name: editor
                            .map(|editor| editor.name.trim())
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned)
                            .or_else(|| existing.and_then(|folder| folder.name.clone())),
                        color: editor
                            .and_then(|editor| editor.color)
                            .or_else(|| existing.and_then(|folder| folder.color)),
                        servers: Vec::new(),
                    });
                    placed_target = true;
                }
                if let Some(entry) = entries.last_mut() {
                    entry.servers.push(guild.to_owned());
                }
                continue;
            }
            // A server the change takes out of the folder goes on its own.
            let folder = server
                .folder
                .as_ref()
                .filter(|folder| target.as_deref() != Some(folder.id.as_str()));
            let id =
                folder.map(|folder| folder.id.trim_start_matches("discord-folder:").to_owned());
            match (entries.last_mut(), id) {
                (Some(entry), Some(id)) if entry.id == id => entry.servers.push(guild.to_owned()),
                (_, id) => entries.push(Folder {
                    id: id.unwrap_or_default(),
                    name: folder.and_then(|folder| folder.name.clone()),
                    color: folder.and_then(|folder| folder.color),
                    servers: vec![guild.to_owned()],
                }),
            }
        }
        entries
    }
}

/// The rail id of the folder an editor changes: its own, or the next number.
fn editor_target(editor: &FolderEditor, servers: &[std::sync::Arc<ChatServer>]) -> String {
    if let Some(id) = &editor.id {
        return id.clone();
    }
    let next = servers
        .iter()
        .filter_map(|server| server.folder.as_ref())
        .filter_map(|folder| folder.id.strip_prefix("discord-folder:"))
        .filter_map(|id| id.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    format!("discord-folder:{next}")
}
