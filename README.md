<p align="center">
  <img src="packaging/icon.svg" width="96" height="96" alt="">
</p>

<h1 align="center">ReCore</h1>

<p align="center">
  <b>A native Discord client with ReCord's terminal look, and a reader for Discord CSS themes.</b><br>
  A fork of <a href="https://github.com/yapfast/yapfast">Yapfast</a>: no browser engine, about 30 MB of RAM idle.
</p>

<p align="center">
  <a href="https://leriart.github.io/ReCore/"><img src="https://img.shields.io/badge/site-live-e94221" alt="Site"></a>
  <a href="https://github.com/leriart/ReCore"><img src="https://img.shields.io/badge/source-leriart%2FReCore-5865F2" alt="Source"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL 3.0 or later"></a>
  <a href="https://github.com/leriart/ReCord"><img src="https://img.shields.io/badge/theme-ReCord-e94221" alt="ReCord"></a>
</p>

ReCore is [Yapfast](https://github.com/yapfast/yapfast) with
[**ReCord**](https://leriart.github.io/ReCord/) as its own look and a CSS
theme interpreter. It talks to Discord's API directly and
draws its own window with [iced](https://iced.rs), so there is no Chromium
anywhere in it. Servers, DMs, threads, forums, friends, voice, screen sharing,
GIFs and the rest of Yapfast's features are here; the fork adds:

- **ReCore and ReCore Paper**, ReCord's signal-orange terminal palette in dark
  and paper daylight, with Pixel Operator and Pixel Operator Mono bundled.
- **An accent picker** that recolours the ReCore themes live.
- **ReCord's call and message sounds**, as a sound set in Settings → Sounds,
  the 78 ReCord pixel glyphs that map onto ReCore's icon set, and square
  framed avatars with square presence glyphs and ReCord's warm, scanlined
grade.
- **A CSS theme interpreter** for Vencord, Vesktop, Equicord, BetterDiscord,
  Legcord and ReCord theme files: colours, accents, fonts, radius, background
  pictures and ReCord's CRT options. Themes are read from ReCore's folder and
  from the clients' own theme folders. See [docs/themes.md](docs/themes.md).

The site is at **https://leriart.github.io/ReCore/**: the landing page, a live
ReCord theme preview, the CSS interpreter notes, the palette and a builder
that downloads a ReCord CSS theme with your settings.

Releases are built by `.github/workflows/release.yml` when a `v` tag is pushed;
the site's Releases section reads GitHub's releases API and shows the assets.
See [docs/packaging.md](docs/packaging.md) for the tag steps and the signing
key.

## Build and run

ReCore builds with Rust 1.97 and the native libraries in
[`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD). The pinned Nix shell is
the easiest way:

```bash
nix develop path:./nix --command cargo run --locked -- --preview
nix develop path:./nix --command cargo run --locked
```

Or, on Linux with rustup and the libraries installed:

```bash
cargo run --locked -- --preview   # the sample conversations, no account
cargo run --locked                # link an account by QR code
```

`--preview` opens the window on Yapfast's fake Discord server. The real client
links an account by scanning Discord's QR code, exactly as upstream does.

## Themes

Settings → Appearance lists the built-in themes, your custom ones, and every
`.theme.css` ReCore can find. **Themes folder** opens ReCore's own folder;
**Reload CSS** rereads the client folders without restarting. Appearance's
note says how many files were read and how many failed.

What a CSS theme can carry into a native window:

| Read | Not painted |
| --- | --- |
| Discord colour tokens, `var()`, `color-mix()`, `light-dark()`, `@import` | Rules that style Discord's DOM (`.messageContent_…`, `[class*=…]`) |
| `.theme-dark` / `.theme-light` and `prefers-color-scheme` | Arbitrary layouts, pseudo-elements and selectors |
| `@font-face` data URIs and local font files, `--font-primary` | CSS `@keyframes` and DOM animations |
| Radius, background picture, ReCord's `--rl-*` colour and CRT options | Client-specific hacks and scripts |

That is the honest boundary of a CSS interpreter in a native renderer: the
palette and the look travel, the web-only rules stay in the file. ReCord's
own CSS is included at [`themes/ReCord.theme.css`](themes/ReCord.theme.css),
and dropping it into the themes folder gives the built-in ReCore colours and
fonts. A CSS theme's background picture is placed under the panels, and
ReCord's scanlines, grain, vignette and corner frame are drawn natively over
the window, with the rolling scan beam and flicker; a CSS theme's DOM
animations are not drawn.

## What comes from Yapfast

Everything Yapfast does, ReCore inherits: Discord and Matrix sign-in, servers,
channels, DMs, group DMs, threads, forums and polls, reactions, Markdown,
custom emoji and stickers, profiles and cosmetics, friends, notifications,
search, pins, server settings, voice and video calls, Go Live, cameras, the
soundboard, keybinds, the tray, updates and the optional Matrix build. The
[upstream README](https://github.com/yapfast/yapfast) and
[upstream docs](https://github.com/yapfast/yapfast/tree/main/docs) describe
them in detail.

ReCore is a fork, not a rewrite. Bug reports about the Discord protocol, voice
or the rest of Yapfast's behaviour belong upstream too when they reproduce
there; ReCore-specific theme problems belong here.

## Configure

The themes and the accent live in Settings → Appearance. The rest of the
window's settings are Yapfast's: appearance of profiles, messages, voice,
keybinds, notifications, advanced and about. `layout.json` keeps the changes
in the data folder:

- Linux: `~/.local/share/recore` (or `$XDG_DATA_HOME/recore`)
- Windows: `%APPDATA%\leriart\ReCore`
- macOS: `~/Library/Application Support/io.github.leriart.ReCore`

The CSS themes folder is `themes` inside that folder.

## Development

```bash
cargo check -p recore-css   # the interpreter, no dependencies
cargo test  -p recore-css
cargo check --workspace
scripts/check --fast
```

The interpreter is `crates/recore-css`; the window's theme plumbing is
`crates/recore-iced/src/themes.rs` and `settings/appearance.rs`; the built-in
palettes are in `crates/recore-core/src/theme.rs`.

## Credits and licences

- **Yapfast**, the client this is forked from, by wiredtenshi and its
  contributors: GPL-3.0-or-later. The full upstream history is in this
  repository.
- **ReCord**, the theme this fork is built around, by leriart: theme code
  Apache-2.0, companion plugin GPL-3.0-or-later. Pixel Operator by Jayvee
  Enaguas (HarvettFox96), CC0 1.0; the licence is in
  `crates/recore-iced-kit/fonts/LICENSE-PixelOperator.txt`.
- Yapfast's bundled palettes and assets keep their own licences, as the
  upstream documentation says.
- ReCore is GPL-3.0-or-later. See [LICENSE](LICENSE).

ReCore is an unofficial fan project. It is not affiliated with Discord Inc.,
Sad Cat Studios or Thunderful. Discord is a trademark of Discord Inc. ReCord's
visual reference is REPLACED; no game assets are included.

## A warning

Third-party clients are against Discord's Terms of Service. In practice
Discord rarely bans people for using one, but if losing your account would be
a disaster, don't use any third-party client. ReCore keeps to the same limits
Discord's own app does and sends no analytics. The login token stays in the
system's credential store and goes only to Discord.
