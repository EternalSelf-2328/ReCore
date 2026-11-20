# ReCore site

The landing page, ReCord theme preview and theme builder, published by
`.github/workflows/pages.yml` to <https://leriart.github.io/ReCore/>.

```bash
# from the repository root
python3 -m http.server 8130 --directory site
# open http://127.0.0.1:8130/
```

- `index.html` — download links for the latest release (Windows, Linux and
  macOS), the live Appearance theme preview with the bundled palettes,
  features, the CSS interpreter and the theme builder.
- `home.js` — fills the download buttons from GitHub's latest release and
  runs the theme preview, including the bundled Catppuccin, Tokyo Night,
  Nord, Gruvbox and Solarized palettes.
- `preview.html` — the ReCord interface preview with sounds.
- `icons.html`, `sounds.html` — ReCord's icon and sound references.
- `theme.css` — ReCord's tokens and Pixel Operator faces for the site itself.
- `themes/ReCord.theme.css` — the full theme the builder downloads and appends
  the chosen settings to.
- `assets/` — ReCord's brand art, fonts and icons (Apache-2.0/CC0).
- `sounds/` — ReCord's signal pack (CC0).

The builder fetches the full theme, appends an `html:root:root` block with the
chosen accent, effects, background and mode, and downloads it as a
`.theme.css` for ReCore's themes folder. If the local copy can't be fetched,
it falls back to the raw file on GitHub.
