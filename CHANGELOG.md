# Changelog

ReCore's current release is shown in Settings > About. The release workflow
uses the same section for the release page, and publishes this file with the
release.

## 0.5.5 - 2026-11-21

### Themes

- CSS themes are interpreted with `style-engine`: selectors, the cascade,
  inheritance, custom properties and state variants now power theme matching.
- A CSS page can lay the window out with Flexbox or Grid. Its rail, sidebar,
  chat and member-list widths, gaps and paint reach the native panels.
- More Discord regions and decorations map to ReCore's own widgets, and the
  rail keeps its own colour beside the sidebar.
- Reading and matching CSS themes and painting their gradients are faster;
  gradients with unpositioned stops stay finite.

### Discord

- Server settings gained Discovery, privacy controls, AutoMod rules,
  integrations, webhooks, soundboard sounds, stage topics and speakers, and
  voice-channel status.
- Joining a server asks its onboarding questions.
- Several accounts can be linked at once, and sign-in with email is supported.
- Group DMs can be renamed and given a new icon.
- Announcements can be crossposted, and super reactions are listed separately.
- Message media, moderation, server folders and the inbox are complete.
- Fenced code is highlighted by its language, and custom emoji can be copied
  as `:name:`.

### Performance and fixes

- Video decoding reuses frames, uses more decode threads and holds its lock
  for less time.
- Cached reactions are read before super reactions.
- Embed icons and thread locks follow Discord again.
- The email login divider spans its row, and the Matrix crosspost command
  keeps its match.
- The preview draws its sample faces and stops failed downloads from retrying.

### Release

- The changelog now appears in Settings > About and on each release page, and
  ships with the Linux, Windows and macOS packages.
