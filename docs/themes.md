# Themes

Settings → Appearance shows the window in one of the themes ReCore comes with,
one of the person's own, or a Discord CSS theme read from a `.theme.css` file.
Each theme is a full set of the colours of `Theme` in
`recore_core::theme` (`crates/recore-core/src/theme.rs`), plus whether it's
dark, which sets the scrollbars' shade and the colours worked out from the
others. The core holds the built-in ones as `BUILT_IN` and reads custom
themes and hex colours; `crates/recore-iced/src/themes.rs` makes the chosen
one the kit's `Theme` (`crates/recore-iced-kit/src/theme.rs`), which every
widget paints with; see [the client's theme](client/window.md#theme). The
choice is kept in `layout.json` as `theme`, the theme's id, and applied
before the window's first frame, and again to a window made anew. With none
kept, the window shows ReCore. An id ReCore doesn't know, or a deleted
theme's, shows ReCore too. The window-control command `theme ID` chooses and
keeps one as Appearance does.

## ReCore and ReCore Paper

The fork's own theme is ReCord's terminal palette: signal orange on warm
charcoal, paper-coloured type, square panels. **ReCore Paper** is the same
palette in daylight. `Appearance → ReCore accent` changes the accent both
derive every shade from; it is kept in `layout.json` as `recore_accent`.
The presets match ReCord's: Amber, Phosphor, Ice, Paper (which switches to
ReCore Paper) and Terminal. ReCord's Pixel Operator and Pixel Operator Mono
are bundled and used by both.

Appearance also carries ReCord's own look options for these two themes, kept
in `layout.json` beside `recore_accent`:

- `recore_scanlines`, `recore_grain` and `recore_vignette` (percentages);
- `recore_beam`, `recore_flicker`, `recore_frame` and `recore_avatar_filter`;
- `recore_background`, `recore_background_dim` and `recore_panel_opacity`;
  `recore_background_filter`, `recore_background_brightness` and
  `recore_background_animate`; the background may be a still picture, an
  animated GIF/APNG/WebP, or a video (MP4, WebM, MOV or MKV), which plays
  through the window's picture players, and keeps playing while nobody is
  using ReCore or the window is out of focus;
- `recore_font` (`pixel`, `mono` or `system`) and `recore_motion` (`full`,
  `subtle` or `off`).

The vignette starts at 0, off: it darkens the window's edges, its top and
bottom most of all.

The window boots with ReCord's signal: a stepped mark, a twelve-step loading
bar, a blinking line and the same screen-in. Menus and dialogs use the
theme's stepped 190 ms motion, a changed room flashes its channel and fades
in over 260 ms, unread rail marks pulse, and speaking rings breathe. These
are native iced animations, not DOM/CSS animations: Discord selectors whose
targets do not exist in a native client are still not painted.

## CSS themes

ReCore reads the themes Vencord, Vesktop, Equicord, BetterDiscord, Legcord
and ReCord itself load. Put a `.theme.css` file in the folder that
**Appearance → Themes folder** opens, or leave it in the client's own themes
folder; ReCore also looks there:

| Client | Folder |
| --- | --- |
| Vencord | `~/.config/Vencord/themes` (or `%APPDATA%\Vencord\themes`) |
| Vesktop | `~/.config/vesktop/themes` |
| Equicord | `~/.config/Equicord/themes` |
| BetterDiscord | `~/.config/BetterDiscord/themes` |
| Legcord | `~/.config/Legcord/themes` |
| ReCore | the data folder's `themes` (the button opens it) |

**Reload CSS** reads them again without restarting. Each theme appears as a
tile with the CSS badge. A theme carries:

- The colours of Discord's tokens: `--background-primary`, `--background-secondary`,
  `--background-floating`, `--channeltextarea-background`, `--text-normal`,
  `--text-muted`, `--header-primary`, `--brand-experiment`, `--status-danger`,
  `--status-positive`, `--text-link`, the border, interactive and input tokens,
  and the newer `--background-base-*`, `--background-surface-*`,
  `--control-*`, `--text-default` names wherever it uses those instead.
- Its own variable chain: `var()`, fallbacks, `!important`, `@import` between
  local files, `@media (prefers-color-scheme: dark|light)`, `.theme-dark`
  and `.theme-light` blocks, `color-mix(in srgb, ...)` and `light-dark()`.
  Hex, `rgb()`, `rgba()`, `hsl()`, `hsla()` and named colours are read.
  A remote `@import` is skipped with a warning: ReCore reads local theme
  files, so download a theme that imports a web base beside it first.
- Its fonts: `@font-face` with a `data:` URI or a file beside the theme is
  loaded into the window's font system, and `--font-primary` picks the
  family.
- Its radius (`--radius-md` or ReCord's `--rl-radius`), a background picture
  or video (`--rl-background-image`, `--background-image`) drawn under the
  panels, and ReCord's CRT options
  (`--rl-motion`, `--rl-icons`, `--rl-scanline-opacity`,
  `--rl-grain-opacity`, `--rl-vignette-opacity`, `--rl-flicker-duration`,
  `--rl-beam-opacity`, `--rl-frame-opacity`, `--rl-avatar-filter`,
  `--rl-avatar-scanline`). The avatar filter and scanline grade the pictures
  the client draws.

ReCord's own `--rl-*` palette is mapped exactly, so dropping
[`themes/ReCord.theme.css`](../themes/ReCord.theme.css) in gives the built-in
ReCore colours and its four Pixel Operator faces.

### Options the theme exposes

Appearance lists the chosen CSS theme's custom properties as controls:

- a colour becomes a swatch with an HSV picker and a hex field;
- an opacity, dimming or surface becomes a slider;
- `on`/`off`, `true`/`false` and ReCord's flicker/beam durations become a
  switch;
- a font and a text variable become a field;
- a `url(...)` picture becomes a field with **Choose…** and **Clear**;
- ReCord's `full`/`subtle`/`off` and background fits become buttons.

The controls are detected from the values, so a theme does not need to
declare them. ReCord's `--rl-*` names get readable labels and groups; other
themes group their colours, typography, shape, pictures and effects. Values
changed there are kept per theme in `layout.json` under `css_options` and the
theme is re-read with them, so its palette and effects change live. **Reset
these options** forgets them; **Show every variable** also lists the
properties ReCore cannot place.

### Native region selectors

Discord themes often style a region directly, not only its variables:

```css
.theme-dark [class*="sidebarList_"] { background: #14161a; }
[class*="chat_"] [class*="message_"]:hover { background: #1b1d21; }
[class*="membersWrap_"] { border-radius: 0; }
```

ReCore gives its native sidebar, guild rail, chat, message area, member list,
composer, header and cards a synthetic node with the class fragments Discord
themes commonly target, then matches the theme's rules against them. It
understands class, id and tag selectors, attribute fragments such as
`[class*="..."]`, descendant selectors, `:is()`, `:not()`, `:where()`, the
`.theme-dark`/`.theme-light` roots and `:hover`/`:focus` states. Specificity,
`!important` and source order are applied, and `var()` values are resolved
before they reach the window. The mapped properties are `background` and
`background-color`, `color`, `border-radius`, `font-family`, `font-size` and
`letter-spacing`.

### What CSS cannot do here

A Discord CSS theme is still written for Discord's web window. ReCore matches
the selectors that name its regions, but it cannot reproduce arbitrary DOM
structure, flexbox or grid layout, pseudo-elements, generated class hashes
without a usable substring, or CSS animations. Those rules stay in the file
without being painted. ReCord's scanlines, grain, vignette and corner frame
are drawn natively over the window, including the rolling scan beam and
flicker; a CSS theme's DOM animations are not drawn. The note under
**Appearance → Theme** says how many files were read and how many could not
be.

## Custom themes

New theme opens an editor on a copy of the theme shown; START FROM swaps it
for a copy of any other. The editor lists every colour by its name
(`Theme::NAMES`, with `-` for `_`), with a swatch and its hex, which takes
`#rgb`, `#rrggbb` or `#rrggbbaa` and marks anything else. Each change shows at
once, so the whole window is the preview. Save keeps the theme and chooses
it; Cancel, leaving Appearance or closing settings shows the theme from
before. A saved theme's card has a pencil that opens it again to rename,
change or delete it; deleting the chosen one shows ReCore. Closing the window
to the [tray](tray.md) closes Settings, so it drops the unsaved edit too.

`layout.json` keeps them in `custom_themes`, in the order Appearance shows
them after the built-in ones:

```json
{
  "id": "custom:1",
  "name": "Plum",
  "base": "recore",
  "dark": true,
  "colours": { "rail": "#111215", "chat": "#2a1a3a", "row-hover": "#1a1b20" }
}
```

`base` is the built-in theme the first copy came from (a copy of a custom
theme takes its base). ReCore writes every colour, but a colour that's
missing, such as one added in a later version, or that isn't a colour, takes
the base's value, and a name ReCore doesn't know is ignored, so themes survive
new colours. Ids are `custom:` and a number, and no built-in id has a colon,
so they can't collide; on reading, ReCore gives a fresh one to any custom
theme whose id isn't of that form or repeats another's. The editor's fields
exist only while it's open; otherwise a custom theme costs its parsed colours
and a card.

| Theme | Id | Dark | Accent | From |
| --- | --- | --- | --- | --- |
| ReCore | `recore` | yes | signal orange | ReCord's terminal palette, the default |
| ReCore Paper | `recore-light` | no | signal orange | ReCord's paper palette |
| Yap | `yap` | yes | marigold | Yapfast's own |
| Bonfire | `bonfire` | yes | blurple | Discord's Dark theme |
| Catppuccin Mocha | `catppuccin-mocha` | yes | mauve | [Catppuccin](https://github.com/catppuccin/catppuccin) |
| Dracula | `dracula` | yes | purple | [Dracula](https://draculatheme.com/contribute) |
| Nord | `nord` | yes | frost nord8 | [Nord](https://www.nordtheme.com/docs/colors-and-palettes) |
| Gruvbox Dark | `gruvbox-dark` | yes | orange | [Gruvbox](https://github.com/morhetz/gruvbox) |
| Tokyo Night | `tokyo-night` | yes | blue | [Tokyo Night](https://github.com/tokyo-night/tokyo-night-vscode-theme), Night style |
| Catppuccin Latte | `catppuccin-latte` | no | mauve | [Catppuccin](https://github.com/catppuccin/catppuccin) |
| Rosé Pine Dawn | `rose-pine-dawn` | no | rose | [Rosé Pine](https://rosepinetheme.com/palette/) |
| Sakura | `sakura` | no | blossom pink | Yapfast's own |

The palettes of Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night and Rosé Pine
are published under the MIT licence; thanks to their authors. For Tokyo
Night, the steps between its grounds and text (bg_highlight, fg_gutter, dark3,
dark5) are named as folke's tokyonight.nvim names them. ReCore takes their
colours, not their names for UI parts, so each mapping is ReCore's:

- The ground steps go rail, sidebar, chat from the deepest to the one text
  sits on, with raised surfaces (popovers, reactions, stage tiles) and the
  composer one step off the chat. Where a palette has no shade below its
  background (Nord's nord0, Tokyo Night's bg_dark), the rail, panel and edge
  darken it, and hover and some row grounds are halfway mixes of two palette
  colours.
- Text, secondary, muted and faint take the palette's text ramp; faint keeps
  at least 2.5:1 against the sidebar.
- Content on the accent and on red (on-accent, on-danger) takes the palette's
  darkest or lightest colour, whichever reads; on pastel accents, as in
  Catppuccin and Tokyo Night, that's dark.
- Light themes wash hovers with a dark overlay (black, or the palette's text
  colour) instead of white, sweep skeletons with a white band, and draw the
  tooltip light, since its text is the theme's text colour.

Bonfire maps Discord's Dark theme from the CSS variables of its web client
(`.theme-dark` with `.theme-darker`, which is what its settings call Dark;
`.theme-dark` alone is Ash, `.theme-midnight` Onyx), read from the public
bundle: the rail and sidebar are `--background-base-lowest` (#121214), the chat
`--background-base-lower` (#1a1a1e), the account panel and composer
`--background-base-low` (#202024), raised surfaces and the tooltip
`--background-surface-high` (#242429), text `--text-default`, muted
`--text-muted`, links `--text-link`, the accent
`--control-primary-background-default` (blurple, #5865f2, hover #4452bb, white
on it), red `--control-critical-primary-background-default` (#d22d39), live
`--status-positive`, and hovers, selections, lines and secondary buttons the
`--interactive-background-*`, `--border-subtle` and
`--control-secondary-background-*` washes laid over their grounds. It carries
no Discord name or logo.

Discord role colours stay as servers set them, and some read faintly on a
light theme; ReCore doesn't adjust them yet.

## Reading a theme's files

The interpreter lives in `crates/recore-css`. `recore_css::load(path)` reads a
file and returns a `ThemeFile` with the resolved `Palette`, `Look`, fonts,
warnings and the metadata from the theme's `/** @name ... */` header. `scan`
walks folders, and `theme_folders` names the common clients' folders.
`Options` chooses the dark or light set, forces an accent, or overrides a
custom property. The crate has no dependencies, so its parser can be tested
on its own:

```bash
cargo test -p recore-css
```
