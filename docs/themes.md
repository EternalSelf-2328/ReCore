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
  through the window's picture players and rests on its first frame while
  the window is unfocused;
- `recore_font` (`pixel`, `mono` or `system`) and `recore_motion` (`full`,
  `subtle` or `off`).

**Background dim** is a palette grade, not a black sheet. Each pixel is
weighted towards the theme's own grounds, inks and signals by its distance
from them, following the Gaussian palette shader in Ambxst's `palette.frag`.
The lookup table is built once per frame at 32 levels per channel, so the
per-pixel work is one index and one mix. The slider (0–100%) mixes the mapped
colour back into the picture; at 0 the picture is untouched. The grade applies
to still frames and animated frames alike, and to CSS themes as well as
ReCore. Moving backgrounds rest while the window is unfocused and play at the
renderer's scale, so a full-window frame is not resampled on the processor on
every tick.

The vignette starts at 0, off: it darkens the window's edges, its top and
bottom most of all.

The window boots with ReCord's signal: a stepped mark, a twelve-step loading
bar, a blinking line and the same screen-in. Menus and dialogs use the
theme's stepped 190 ms motion, a changed room flashes its channel and fades
in over 260 ms, unread rail marks pulse, and speaking rings breathe. These are native iced animations. CSS animations on the page or body are
read as a quiet native ground wash; Discord selectors whose targets do not
exist in a native client are still not painted.

## Ambxst

[`themes/Ambxst.theme.css`](../themes/Ambxst.theme.css) is a Discord theme
inspired by [Axenide's Ambxst](https://github.com/Axenide/Ambxst): its exact
default dark adapter (`#1a1111` ground, `#302625` surface, rose `#ffb3ae`)
and a paper-light counterpart, 16 px rounded panels and Roboto Condensed.
It is a complete theme for Vencord, Vesktop, Equicord, BetterDiscord and
Legcord as well as a CSS theme ReCore reads: the same file carries every
Material role from `Colors.qml`, all nineteen `sr*` surface variants from
Ambxst's default `theme.json`, Discord's semantic tokens, ReCore's `--rl-*`
palette and the native region rules. It turns ReCord's CRT layer off and keeps
the window's rounded, shadowed Material surfaces. Its options cover the
Material roles, surface gradients and borders, opacity and transparency, blur
and shadows, gradients, roundness, type sizes, bar, dock, notch, overview,
context-menu, action-grid and notification geometry. Drop it in the themes
folder like any other `.theme.css` file; it has no remote imports. The
package carries it, so a fresh install has it without downloading anything.
[docs/ambxst.md](ambxst.md) is the source-by-source inventory, including the
nine Ambxst presets and the parts of the Quickshell shell a Discord window
cannot paint.

## Bundled themes

Every package carries the themes in `themes/`: ReCore and ReCore Paper,
Ambxst, Catppuccin Mocha, Tokyo Night, Nord Aurora, Gruvbox Ember and
Solarized Paper. ReCore looks for the package's folder beside the binary, in
the app's `share/recore/themes`, and in a macOS bundle's
`Contents/Resources/themes`; the data folder's own `themes` folder comes first,
so a person's copy wins. No theme needs a remote download to show up.

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
| Equibop | `~/.config/Equibop/themes` |
| BetterDiscord | `~/.config/BetterDiscord/themes` |
| Legcord | `~/.config/Legcord/themes` |
| Dorion | `~/.config/Dorion/themes` |
| GoofCord | `~/.config/GoofCord/themes` |
| Replugged | `~/.config/replugged/themes` |
| ReCore | the data folder's `themes` (the button opens it) |

**Reload CSS** reads them again without restarting. It follows theme packs in
subfolders, leaves hidden folders and dependency trees alone, and reads the
same theme once when it is copied into several client folders. While
Settings is open, a theme saved by an editor outside ReCore reloads itself.
The **Theme URL** field downloads a theme into ReCore's folder, and the tile
turns up ready to choose. Each theme appears as a tile with the CSS badge. A
theme carries:

- The colours of Discord's tokens: `--background-primary`, `--background-secondary`,
  `--background-floating`, `--channeltextarea-background`, `--text-normal`,
  `--text-muted`, `--header-primary`, `--brand-experiment`, `--status-danger`,
  `--status-positive`, `--text-link`, the border, interactive and input tokens,
  and the newer `--background-base-*`, `--background-surface-*`,
  `--control-*`, `--text-default` names wherever it uses those instead.
- Its own variable chain: `var()`, fallbacks, `!important`, `@import` between
  local files, `@media (prefers-color-scheme: dark|light)`, `.theme-dark`
  and `.theme-light` blocks, `color-mix()` (two or more colours, with
  `in srgb` or `in srgb-linear`), `light-dark()`, `hwb()`, `oklab()` and
  `oklch()`, `lab()`, `lch()` and `color(srgb ...)`/`color(display-p3 ...)`.
  Hex, `rgb()`, `rgba()`, `hsl()`, `hsla()` and named colours are read, and
  DiscordRecolor's bare `30,30,30` RGB triples are understood as colours.
  The cascade keeps specificity, source order, `!important` and `@layer`
  order across the root, theme-class and media-query blocks, so a later
  `:root` declaration can win over an earlier `.theme-dark` one, as in a
  browser. `@supports` conditions and the media/supports condition after an
  `@import` are applied, and `@property` gives a variable its syntax and
  initial value.
  An `http:` or `https:` `@import` is downloaded, with its own relative
  imports; so are a remote font in `@font-face` and a remote picture in
  `--background-image`. Downloads are kept for ten minutes in memory and on
  disk, so a theme works offline after its first load; **Reload CSS**
  forgets them, so a changed base theme is read at once. A file over 8 MB, a
  failed download or an offline machine keeps the rest of the theme and says
  so in Appearance's note. A theme's remote imports are fetched when the
  theme is chosen, so a folder full of remote themes does not stall the
  first frame.
- Its fonts: `@font-face` with a `data:` URI, a file beside the theme or a
  remote URL is loaded into the window's font system, and `--font-primary`
  picks the family.
- Its radius (`--radius-md` or ReCord's `--rl-radius`), a background picture,
  CSS gradient or video (`--rl-background-image`, `--background-image`, or a
  `body` / `body::before` background rule) drawn under the panels, its fit
  (`--rl-background-size`: `cover`, `contain` or `100% 100%`), how opaque the
  panels are over it (`--rl-panel-opacity` or `--rl-surface`), and ReCord's
  CRT options
  (`--rl-motion`, `--rl-icons`, `--rl-scanline-opacity`,
  `--rl-grain-opacity`, `--rl-vignette-opacity`, `--rl-flicker-duration`,
  `--rl-beam-opacity`, `--rl-frame-opacity`, `--rl-avatar-filter`,
  `--rl-avatar-scanline`). The avatar filter and scanline grade the pictures
  the client draws. A `@keyframes` animation on `:root` or `body` whose steps
  change `background`, `background-color`, `color` or `opacity` is sampled by
  the window and painted as a quiet moving ground; finite animations stop,
  infinite ones repeat, and `alternate`/`reverse` are understood. Other
  animations remain in the file.

ReCord's own `--rl-*` palette is mapped exactly, so dropping
[`themes/ReCord.theme.css`](../themes/ReCord.theme.css) in gives the built-in
ReCore colours and its four Pixel Operator faces.

### Options the theme exposes

Appearance lists the chosen CSS theme's custom properties as controls, in
collapsible groups, with a filter field for large themes (Catppuccin exposes
more than 250, and DiscordRecolor's imported base more than 700) and
**Show every variable** for the ones ReCore can't place:

- a colour becomes a swatch with an HSV picker and a hex field, keeping an
  eight-digit colour's alpha;
- an opacity, dimming or surface becomes a slider;
- `on`/`off`, `true`/`false` and ReCord's flicker/beam durations become a
  switch;
- a font and a text variable become a field;
- a `url(...)` picture becomes a field with **Choose…** and **Clear**,
  including one the theme embeds as a data URI;
- a BetterDiscord `@var slider` becomes a slider with its min/max/step;
- ReCord's `full`/`subtle`/`off`, background fits and choices listed in a
  comment become buttons.

Each option shows its theme default when it names one, a **Reset this
option** button once it changes, and a badge when it exists only in the dark
or light palette. **Only recommended** hides unannotated variables and
internal aliases; **Only changed** shows just what was overridden;
**Export changes** writes those overrides as a small `.theme.css` file that
can be loaded after the theme elsewhere.

The controls are detected from the values and named from what the theme says
about them. ReCore reads the variable name, an inline comment beside the
declaration, and the heading above it (`/* text colors */`, `/* ACCENT
COLORS */`); both Midnight/system24 and ClearVision give much better labels
this way. It also understands BetterDiscord's `@var` header (`checkbox`,
`text`, `color`, `select` and `slider`), matching names such as
`accentcolor_s` to the `--accentcolor` property. A comment that lists choices (`off: default
position, hide: hide buttons, titlebar: move it`) turns the option into
buttons. This is automatic: a theme needs no ReCore-specific metadata.

ReCord's `--rl-*` names get curated labels and groups. Other themes are
grouped as backgrounds, text, channels, accents, status, borders, interface,
base colours, typography, shape, pictures and effects, with a fallback group
for anything ReCore cannot place. Values changed there are kept per theme in
`layout.json` under `css_options` and the theme is re-read with them, so its
palette and effects change live. **Reset these options** forgets them;
**Show every variable** also lists the properties ReCore cannot place.

For predictable labels, theme authors can put a short description after a
declaration (`--main-color: #2780e6; /* main accent colour */`), group a
block under a heading comment, and use BetterDiscord `@var` metadata for
labels and types. ReCore still reads the CSS when none of those are present.

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
`[class*="..."]`, descendant selectors, `:is()`, `:not()`, `:where()`, simple
`:has()` descendant checks, the `.theme-dark`/`.theme-light` roots and
`:hover`/`:focus` states. Specificity,
`!important` and source order are applied, and `var()` values are resolved
before they reach the window. A bare tag selector or `*` never styles a
region on its own: it needs a class, an id or an attribute, so a theme's
`div { ... }` reset cannot paint the rail. Fully transparent backgrounds are
left to the region's own colour instead of erasing it, and a radius on a
button, a card or a hover state does not redefine the whole window's shape.
The mapped properties are `background`, `background-color` and
`background-image`, `color`, `border-radius`, `font-family`, `font-size` and
`letter-spacing`. A gradient's first opaque stop is used as the region's
colour where the native window cannot paint the full gradient.

### What CSS cannot do here

A Discord CSS theme is still written for Discord's web window. ReCore matches
the selectors that name its regions, but it cannot reproduce arbitrary DOM
structure, flexbox or grid layout, pseudo-elements, generated class hashes
without a usable substring, or element-level CSS animations. Those rules stay
in the file without being painted. A root/body animation of the ground is the
exception: its colour and opacity keyframes become a quiet native wash, and a
gradient background is painted at the background size. ReCord's scanlines,
grain, vignette and corner frame are drawn natively over the window, including
the rolling scan beam and flicker, which rest while the window is not focused.
The note under **Appearance → Theme** says how many files were read and how
many could not be.

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

A few things that Discord's client draws off its palette, Bonfire draws in
the inks measured from it (`Theme::bonfire` in the core, `bonfire` on the
kit's theme): a mention's text, the channel list's names and glyphs, a
message's time, the header's search box and the Jump to present pill. A
custom theme made from Bonfire keeps them while its link colour is
Bonfire's; every other theme draws those from its own colours.

Discord role colours stay as servers set them, and some read faintly on a
light theme; ReCore doesn't adjust them yet.

## Reading a theme's files

The interpreter lives in `crates/recore-css`. `recore_css::load(path)` reads a
file and returns a `ThemeFile` with the resolved `Palette`, `Look`, fonts,
warnings and the metadata from the theme's `/** @name ... */` header.
`load_with_imports` and `load_str_with_imports` take a fetch callback for a
theme's remote files, and `scan_with_imports` does so while walking folders;
`scan` and `load` are the same without remote access. `theme_folders` names
the common clients' folders. `Options` chooses the dark or light set, forces
an accent, or overrides a custom property. The crate has no dependencies, so its parser can be tested
on its own.

A theme is data, never code. A damaged or hostile file is read as far as it
makes sense and the rest is left out: unterminated blocks, strings and
comments, NUL bytes, invalid numbers, reference loops and a variable that
names itself twice at every level (which would be 2^n substitutions) all
end without a panic, a stack overflow or a run that never returns. A value
that would grow past 1 MiB or read more than 4096 variables is treated as
missing. `crates/recore-css/tests/robust.rs` cuts every bundled theme at
120 places and damages each with random fragments, and checks the hostile
inputs by name.

```bash
cargo test -p recore-css
```

To see what ReCore read from a theme by hand:

```bash
cargo run -p recore-css --example read -- themes/Ambxst.theme.css
```
