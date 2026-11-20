# Themes

`ReCord.theme.css` is ReCord's self-contained theme, copied from
[leriart/ReCord](https://github.com/leriart/ReCord) at version 4.3.1. It is
here as the reference CSS theme for ReCore's interpreter and as the source of
ReCord's palette and fonts. The built-in **ReCore** and **ReCore Paper**
themes in `crates/recore-core/src/theme.rs` are the same palette without the
web-only rules.

`Ambxst.theme.css` is a theme inspired by
[Axenide's Ambxst](https://github.com/Axenide/Ambxst): Ambxst's exact default
warm Material You roles, all nineteen `sr*` surfaces, rose accent, 16 px
rounded panels and Roboto Condensed, in dark and paper-light. It mirrors the
shell's `config/defaults/theme.js`, `Colors.qml`, `StyledRect.qml` and the
component geometry as options, and works in Discord's browser clients and in
ReCore, where it also turns ReCord's CRT layer off.

The other files are self-contained themes ReCore ships and the site previews:
**Catppuccin Mocha**, **Tokyo Night**, **Nord Aurora**, **Gruvbox Ember** and
**Solarized Paper**. Each has a dark or light palette, native region rules and
the `--rl-*` options Appearance shows; none needs a remote import.

ReCore reads every `.css` file in its themes folder, including theme packs in
subfolders, so drop any Vencord, Vesktop or Discord theme here (or in the
client's own folder) and press **Reload CSS** in Settings → Appearance.
Appearance can also import a theme from a URL, notices a file saved while
Settings is open, and names each custom property from its variable name, the
comment beside it and the section heading above it; `@property` rules and
BetterDiscord `@var` labels and types are read too, and a comment that lists
choices (`off: ... , hide: ...`) becomes buttons. What is read and what is not
is in [docs/themes.md](../docs/themes.md). The full inventory of Ambxst's
colours, surface variants, component geometry and preset differences is in
[docs/ambxst.md](../docs/ambxst.md).

ReCord's theme is Apache-2.0, by leriart; the licence is in
[`LICENSE-Apache-2.0.txt`](LICENSE-Apache-2.0.txt). Pixel Operator is CC0 by Jayvee Enaguas
(HarvettFox96); its licence is in
`crates/recore-iced-kit/fonts/LICENSE-PixelOperator.txt`. Ambxst itself is
AGPL-3.0; `Ambxst.theme.css` is ReCore's own file, inspired by its palette and
does not contain Ambxst's code.
