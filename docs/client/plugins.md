# Vencord and Vesktop plugins, in ReCore

Vesktop is Discord's web app in Electron with Vencord on top. Its plugins
are JavaScript patches to Discord's DOM, React tree and bundled modules, so
they cannot be loaded into ReCore: the window is iced and the client talks
to Discord directly. ReCore reimplements the behaviour it wants natively,
where a feature fits a native client and Discord's protocol allows it.

The plugin list is <https://vencord.dev/plugins>. The useful ones fall into
three groups.

## Already native

These Vencord/Vesktop plugins describe things ReCore already does, often in
its own shape:

| Plugin | In ReCore |
| --- | --- |
| CallTimer | The call's clock in the dock and the channel row. |
| GameActivityToggle | Settings → Activity → Share your activity, and the dock's game button. |
| MemberCount | Member and online counts in the header and the member list. |
| NoBlockedMessages | Settings → Messages → Hide blocked messages. |
| NoReplyMention | The reply bar's `@ On`/`@ Off`, and the Messages switch. |
| PinDMs | Pinned DMs, reordered from their menu. |
| Rich Presence / WebRichPresence | The Linux Discord IPC socket (`docs/discord/people.md`). |
| SilentTyping | Settings → Messages → Let others see when you're typing. |
| TypingIndicator | Who is typing above the composer. |
| UserVoiceShow | Who is in a voice channel, on the channel row and in DMs. |
| ValidReply | Replies that can't be loaded say so. |

## Added here

| Plugin | In ReCore |
| --- | --- |
| BetterGifPicker | Settings → Messages → Open GIFs in Favorites. |
| PlatformIndicators | Discord's phone and globe marks replace the presence dot for someone online only on a phone or only on the web. |
| CharacterCounter | The composer's `used/2000` count, red past Discord's limit. |
| PlainFolderIcon | Settings → Messages → Plain folder icons. |
| ReverseImageSearch | A picture's menu → Search Image → Google Lens, Yandex Images or TinEye. |
| ImageZoom | Wheel zooms and a drag moves a picture in its viewer, kept inside its box; each picture starts unzoomed. |
| SilentMessageToggle | The bell in the composer puts Discord's `@silent` at the start, and takes it off. |
| StreamerMode | Settings → Advanced silences interface sounds and notifications and hides this account's name and tag. |
| Translate | A message's menu → Translate shows Google's answer in a card; Settings → Messages picks the languages written in and sent in, the composer's globe turns translation before sending on and off, and a failed translation sends the words as written. |

The settings are kept in `layout.json` like the rest (`plain_folders`,
`gif_favorites_first`). `@silent` is Discord's own: the core strips it from
the sent message and sends with no notification.

## Not suitable, or still open

- Plugins that patch Discord's DOM, CSS, React, console or bundler
  (`ConsoleJanitor`, `FixDiscordCss`, `ThemeAttributes`, `WebContextMenus`,
  `WebKeybinds`, `WebScreenShare`, `Experiments`, `NoDevtoolsWarning`,
  `ShikiCodeblocks`, and the like) have no nodes here to patch. ReCore's CSS
  reader paints the colours, type and shape a theme names in
  [`docs/themes.md`](../themes.md).
- Plugins that depend on Discord's internal stores, hidden permissions or
  experiments (`ShowHiddenChannels`, `PermissionsViewer`, `Experiments`,
  `FakeNitro`) would either need protocol the client doesn't have or would
  encourage behaviour Discord's terms don't allow. ReCore lists channels it
  may not read with a lock instead.
- Third-party services (`ReviewDB`, `USRBG`, `Dearrow`, `MusicRichPresence`
  through Last.fm/ListenBrainz) need an account or keys and network requests
  of their own; none is built in. Translate uses Google's public endpoint
  without a key; DeepL and Kagi, which need one, are not offered.
- Good native candidates still to do: `MessageLogger` (edited and deleted
  message content), `ExpressionManager` (cloning emoji and stickers),
  `ViewIcons` (a profile picture or banner at full size), `CustomRPC` (an
  activity typed in by hand), `SpotifyControls` (play, pause and skip from
  the account panel), `MessageLinkEmbeds`, `ImageFilename` and
  `HideMedia`.

A plugin is added like any other feature: the core's state and protocol in
`recore-core`/`recore-discord`, then its rows or controls in
`recore-iced`; see [adding a feature](../discord/adding-a-feature.md) and
[the client's structure](structure.md).
