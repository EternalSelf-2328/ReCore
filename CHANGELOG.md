# Changelog

ReCore's current release is shown in Settings > About. The release workflow
uses the same section for the release page, and publishes this file with the
release.

## 0.5.6 - 2026-11-23

### Themes

- CSS pages are painted with `style-engine`'s complete tiny-skia sink, so
  radial and conic gradients, shadows, borders, clips, transforms, opacity,
  filters and blend modes reach the window instead of being discarded.
- Appearance can create a CSS theme from a complete starter file and edit any
  CSS theme's source. Background pictures and ReCord's scanlines, grain and
  vignette can be added from the editor, and **Save CSS** shows the result at
  once.
- Escape and **Back to themes** leave the theme editor before they close
  Settings, so entering and leaving the menu no longer loses the page under
  the editor.
- Reading a CSS theme's source keeps its comments, imports, fonts and effects;
  a file the reader rejects stays in the editor with its error instead of being
  written.

### Calls

- This account's speaking ring follows the processed microphone's voice rather
  than the transmit gate: it lights when speech is heard, stays through short
  pauses and turns off 220 ms after the voice stops. Push to talk no longer
  keeps it lit during a silent key press, and muting hides it.

### Fixes

- The page paint image now covers every operation the CSS engine produces,
  including groups and clips that used to disappear.
- Theme errors and unreadable files are reported without closing the editor or
  the settings page.

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
