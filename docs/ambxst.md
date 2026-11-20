# Ambxst style inventory

This note records how [`themes/Ambxst.theme.css`](../themes/Ambxst.theme.css)
was derived from [Axenide's Ambxst](https://github.com/Axenide/Ambxst). It is
an inventory of the shell's visual system, not a copy of its QML: ReCore has
no Quickshell, no Wayland layershell, no dock or notch. The parts that can be
represented in a Discord CSS theme are carried as `--ambxst-*` properties and
native selectors; the rest is exposed as documented Appearance options.

## Sources

| Source | What it defines |
| --- | --- |
| `modules/theme/Colors.qml` | Material colour roles, OLED/light switches, semantic aliases (`warning`, `success`), critical colours |
| `modules/theme/Styling.qml` | Per-surface variants, radius and font-size offsets, item-colour lookup |
| `modules/components/StyledRect.qml` | Linear/radial gradients, halftone, border, opacity, shadow, per-surface item colour |
| `modules/components/Shadow.qml`, `Outline.qml` | Shadow offsets/opacity/blur and a hard edge |
| `modules/components/BarPopup.qml`, `OptionsMenu.qml`, `SearchInput.qml` | Popup, menu and search geometry |
| `modules/components/StyledSlider.qml`, `ToggleButton.qml`, `SegmentedSwitch.qml` | Slider, switch and segmented-control geometry and interaction colours |
| `modules/notifications/*` | Notification width, padding, icon sizes and critical style |
| `modules/bar/BarBg.qml`, `BarBgShadow.qml`, `modules/dock/*` | Bar/dock corners, margins, opacity and shadows |
| `modules/notch/*`, `modules/widgets/overview/*`, `modules/lockscreen/*`, `modules/desktop/*` | Shell-only surfaces and geometry |
| `modules/theme/DiscordGenerator.qml` | The shell's own DiscordRecolor token generator |
| `config/defaults/*.js`, `assets/presets/*/theme.json` | Defaults and the nine built-in presets |

## The colour roles

Ambxst's default dark adapter is this Material You rose scheme:

| Role | Value | Role | Value |
| --- | --- | --- | --- |
| `background` | `#1a1111` | `surface` | `#302625` |
| `surfaceBright` | `#453a3a` | `surfaceDim` | `#1a1111` |
| `surfaceContainer` | `#271d1d` | `surfaceContainerLow` | `#231919` |
| `surfaceContainerLowest` | `#140c0c` | `surfaceContainerHigh` | `#322827` |
| `surfaceContainerHighest` | `#3d3231` | `surfaceVariant` | `#534342` |
| `primary` | `#ffb3ae` | `primaryContainer` | `#733331` |
| `primaryFixed` | `#ffdad7` | `primaryFixedDim` | `#ffb3ae` |
| `secondary` | `#e7bdb9` | `secondaryContainer` | `#5d3f3d` |
| `tertiary` | `#e2c28c` | `tertiaryContainer` | `#594319` |
| `error` | `#ffb4ab` | `errorContainer` | `#93000a` |
| `overBackground` | `#f1dedd` | `overSurfaceVariant` | `#d8c2c0` |
| `outline` | `#a08c8b` | `outlineVariant` | `#534342` |
| `blue` | `#cebdfe` | `blueContainer` | `#4c3e76` |
| `cyan` | `#84d5c4` | `cyanContainer` | `#005045` |
| `green` | `#b7d085` | `greenContainer` | `#3a4d10` |
| `yellow` | `#dec56e` | `yellowContainer` | `#554500` |
| `magenta` | `#fcb0d5` | `magentaContainer` | `#6c3353` |
| `white` | `#82d3e0` | `whiteContainer` | `#004f58` |

`surface` and `surfaceBright` in `Colors.qml` are not literal constants: they
are `Qt.tint(background, overBackground, 0.1)` and `0.2`. The values above are
the resulting sRGB colours (`#302625` and `#453a3a`). The theme includes every
role from the adapter, including the `over*`, `*Fixed`, source/value and
`inverse*` families, as individual options.

The light block is the paper-rose counterpart ReCore already shipped. Ambxst's
light mode normally comes from a generated wallpaper palette; ReCore keeps a
fixed light palette so a theme does not depend on a wallpaper being present.

## Surface variants

Every `sr*` object in `config/defaults/theme.json` is a complete style, not
just a colour. `StyledRect.qml` reads:

- a gradient stop list (up to eight stops), or a single colour;
- gradient type `linear`/`radial`/`halftone`;
- angle, radial centre, halftone dot min/max and range;
- a border colour and width;
- surface opacity;
- an item/text colour.

The default variant set and its border/opacity is:

| Variant | Fill | Border | Opacity | Item |
| --- | --- | ---: | ---: | --- |
| `bg` | `background` | `surfaceBright` 0 | 100% | `overBackground` |
| `popup` | `background` | `surfaceBright` 2 | 100% | `overBackground` |
| `internalBg` | `background` | `surfaceBright` 0 | 100% | `overBackground` |
| `barBg` | `surfaceDim` | `surfaceBright` 0 | 0% | `overBackground` |
| `pane` | `surface` | `surfaceBright` 0 | 100% | `overBackground` |
| `common` | `surface` | `surfaceBright` 0 | 100% | `overBackground` |
| `focus` | `surfaceBright` | `surfaceBright` 0 | 100% | `overBackground` |
| `primary` | `primary` | `primary` 0 | 100% | `overPrimary` |
| `primaryFocus` | `overPrimaryContainer` | `overBackground` 0 | 100% | `overPrimary` |
| `overPrimary` | `overPrimary` | `overPrimary` 0 | 100% | `primary` |
| `secondary` | `secondary` | `secondary` 0 | 100% | `overSecondary` |
| `secondaryFocus` | `overSecondaryContainer` | `overBackground` 0 | 100% | `overSecondary` |
| `overSecondary` | `overSecondary` | `overSecondary` 0 | 100% | `secondary` |
| `tertiary` | `tertiary` | `tertiary` 0 | 100% | `overTertiary` |
| `tertiaryFocus` | `overTertiaryContainer` | `overBackground` 0 | 100% | `overTertiary` |
| `overTertiary` | `overTertiary` | `overTertiary` 0 | 100% | `tertiary` |
| `error` | `error` | `error` 0 | 100% | `overError` |
| `errorFocus` | `overBackground` | `overBackground` 0 | 100% | `overError` |
| `overError` | `overError` | `overError` 0 | 100% | `error` |

The original file expresses these with `gradientType`, `gradientAngle`,
`gradientCenterX/Y`, `halftoneDotMin/Max`, `halftoneStart/End`,
`halftoneDotColor` and `halftoneBackgroundColor`. The CSS theme carries each
as `--ambxst-sr-<variant>-*` so Appearance can change a surface without
editing the stylesheet. The native window uses the first opaque stop where it
cannot paint a gradient; browser clients receive the gradient itself.

## Geometry and component styles

Constants read from the components and defaults:

| Group | Values |
| --- | --- |
| Global shape | radius 16 px; corners on; animation 300 ms; shadow opacity 0.5, blur 1, offsets 0 |
| Bar | 44 px total (36 px controls + 2×4 px), 4 px outer margin, corner cut-out roundness + 4, `pillStyle: default`, transparent `barBg`, frame off |
| Dock | 48 px height, 24 px icons, 4 px spacing/margin, 16 px hover strip, 32 px shadow room, corner radius roundness + 4 |
| Popup | 8 px padding, 8 px visual margin, 16 px shadow margin, 220×150 px default content, scale 0.9, radius roundness + 8 |
| Menu | 140 px default width (120–300), 36 px rows, 8 px padding, 2 px border, radius roundness, item radius roundness − 6 |
| Search | 48 px height, 8 px padding, radius roundness + 4, 20 px icon, 32 px prefix |
| Slider | 4 px track, 4×16 px handle (2×20 while dragging), 4 px handle spacing |
| Toggle | 36×36 px, 18 px icon |
| Segmented | 28 px cells, 2 px padding/spacing, radius roundness − 4, highlight roundness − 6 |
| Notifications | 350 px stack, 8 px margin/padding/gap, 32/48 px icons, 8 px radius |
| Overview | 2 rows × 5 columns, 0.15 scale, 8 px spacing |
| Notch | 44 px default height, radius roundness + 4 (roundness + 20 expanded) |
| Context menu | 160 px width, 32 px rows |
| Action grid | 48 px buttons, 20 px icons, 4 px spacing, 3 columns, 8 px text spacing |
| Circular controls | 48 px dial, 6 px handle spacing, 8 px handle, 4 px line, 45° gap |
| Carousel | 4 px dots, 6 px spacing |
| Separator | 1 px line at 10% opacity, radius roundness |
| Compositor | 2 px active/inactive edge, 2/4 px inner/outer gaps, 8 px shadow range, 4 px blur, 2 passes |

These are options prefixed `--ambxst-` in the theme. The desktop shell's
bar/dock/notch/lockscreen itself cannot be recreated by a Discord client, but
the values are still available for documentation, for the site preview and
for an eventual native shell.

## Effects and type

The shell is more than flat rectangles:

- **`linear_gradient.frag`** interpolates up to eight stops along an angle;
  angle 0 runs vertically from top to bottom. **`radial_gradient.frag`**
  interpolates the same stops from a normalised centre with a smooth edge.
- **`halftone.frag`** builds a rotated dot grid; dot radius shrinks from
  `halftoneDotMax` to `halftoneDotMin` between `halftoneStart` and
  `halftoneEnd`, using a separate dot and ground colour. The cell is twice the
  maximum dot diameter.
- **`wavyline.frag`** and **`circular_wavy.frag`** drive the player
  visualiser; **`unified_pass1/2.frag`** perform the two-pass horizontal/vertical
  blur used for shadows, borders and masks; **`palette.frag`** mixes a
  background toward the palette rather than laying a flat dim over it.
- **`StyledRect.qml`** clips children to the radius (`ClippingRectangle`),
  animates radius over `animDuration / 4`, applies a `MultiEffect` shadow, and
  paints its border as a separate overlay so children never cover it.
- **`Icons.qml`** uses the **Phosphor-Bold** icon font; `Config.theme.font` is
  Roboto Condensed and `Config.theme.monoFont` is Iosevka Nerd Font Mono.

The CSS file carries the colours, stops, dot sizes, ranges and geometry of
these effects. It cannot run Qt shaders, so a browser theme approximates the
halftone and glass with ordinary CSS backgrounds or leaves the surface flat.
The native ReCore window uses its own iced renderer and reads the same palette
and region rules; it does not pretend to be a Quickshell compositor.

## Presets

Ambxst ships nine presets. Their `theme.json` files differ mostly in surface
gradients, roundness, font and light/OLED mode:

| Preset | Mode | Shape | Signature |
| --- | --- | --- | --- |
| Ambxst Default | dark | 16 px, Roboto Condensed 14 | Flat Material rose surfaces, transparent bar |
| Caelestiax | dark | 16 px | Opaque bar/pane surfaces, same rose family |
| Dotsquared | dark | 0 px, Terminus 8 | Halftone/surfaceBright, 1 px borders, square |
| Frutiger Aero | light | 16 px | Radial glass gradients, 2 px borders, opacity 0.5–0.8 |
| Frutiger Aqua | dark | 16 px | Blue literal gradients, 2 px borders, glossy overlays |
| GNOME | dark/OLED | 16 px | Flat black OLED surfaces, high contrast |
| Liquid Glass | dark/OLED | 14 px | Low-opacity radial glass, 0.2–0.5 surfaces, blur |
| Manga | light/OLED | 16 px | Halftone paper surfaces |
| Retro | light/OLED | 0 px, Terminus 8 | Halftone, 1 px outlines, square |

The bundled theme implements the **Ambxst Default** style with the complete
colour and surface system. Other presets remain reference data: they change
the surface gradients and geometry, not just a colour, so shipping each one
as a separate `.theme.css` would be a new theme rather than an option toggle.

## Built-in colour schemes

Ambxst also carries wallpaper-generated colour schemes in `assets/colors/`:
Ayu, Catppuccin, Everforest, GitHub, Gruvbox, Kanagawa, Nord, Paradise,
Posterpole, Rose Pine, Tokyonight and Yoru, each with a `dark.json` and a
`light.json`. These are **palettes**, not surface styles: they replace the
Material roles while the selected preset's gradients and geometry stay. ReCore
uses the upstream rose adapter as the theme's fixed palette so the stylesheet
looks the same without a wallpaper generator; a user who wants one of these
palettes can override the `--ambxst-*` colour roles in Appearance.

## Mapping to Discord and ReCore

The theme uses three layers:

1. **Material roles** (`--ambxst-*`) and all surface variants
   (`--ambxst-sr-*`). These are the source of truth and become Appearance
   controls.
2. **Discord semantic tokens** (`--background-primary`, `--text-normal`,
   `--brand-experiment`, `--control-*`, `--border-*`, old DiscordRecolor
   names). They point at the Material roles so a browser client and ReCore
   agree.
3. **Native region rules** for the rail, sidebar, chat, member list, header,
   composer, cards, menus and rows. ReCore cannot render a gradient for every
   control, so a region takes the first opaque stop of its `sr*` surface.

The shell-only effects are deliberately not faked: Quickshell's
`MultiEffect` blur, the bar/dock/notch geometry, the ShaderEffect halftone
painter and the compositor's shadow are not part of a Discord window. The CSS
theme exposes their values but does not pretend to paint the shell.
