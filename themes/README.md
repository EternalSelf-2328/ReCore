# Themes

`ReCord.theme.css` is ReCord's self-contained theme, copied from
[leriart/ReCord](https://github.com/leriart/ReCord) at version 4.3.1. It is
here as the reference CSS theme for ReCore's interpreter and as the source of
ReCord's palette and fonts. The built-in **ReCore** and **ReCore Paper**
themes in `crates/recore-core/src/theme.rs` are the same palette without the
web-only rules.

ReCore reads every `.css` file in its themes folder, so drop any Vencord,
Vesktop or Discord theme here (or in the client's own folder) and press
**Reload CSS** in Settings → Appearance. What is read and what is not is in
[docs/themes.md](../docs/themes.md).

The theme is Apache-2.0, by leriart; the licence is in
[`LICENSE-Apache-2.0.txt`](LICENSE-Apache-2.0.txt). Pixel Operator is CC0 by Jayvee Enaguas
(HarvettFox96); its licence is in
`crates/recore-iced-kit/fonts/LICENSE-PixelOperator.txt`.
