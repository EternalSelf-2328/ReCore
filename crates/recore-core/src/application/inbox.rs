//! The inbox: the messages that mentioned this account, and those it saved
//! for later.

use super::*;

impl Application {
    /// Saves an item for later, or takes it out, as the message menu's
    /// bookmark does.
    pub(super) fn toggle_saved(&mut self, item: String) -> Vec<Effect> {
        let Some(room) = self.state.room_of(&item).cloned() else {
            return vec![];
        };
        if self.state.preview
            || Network::of(&room) != Network::Discord
            || self.state.message(&item).is_none()
        {
            return vec![];
        }
        let save = !self.state.saved_items.contains(&item);
        // The menu changes at once; Discord's answer confirms it.
        if save {
            self.state.saved_items.insert(item.clone());
        } else {
            self.state.saved_items.remove(&item);
        }
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SaveMessage { room, item, save },
        ))]
    }

    /// Takes a mention out of the inbox, as Discord's client does.
    pub(super) fn dismiss_mention(&mut self, item: String) -> Vec<Effect> {
        let Some(id) = item.strip_prefix("d:").map(str::to_owned) else {
            return vec![];
        };
        if let Some(mentions) = &mut self.state.recent_mentions {
            mentions.retain(|mention| mention.item != item);
        }
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::DeleteMention(id),
        ))]
    }
}
