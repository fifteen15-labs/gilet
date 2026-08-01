# Gilet mark

`logo.svg` is a hooded coat, flat-sketched front-on the way a technical
outerwear spec sheet would draw it: a body that flares from the
shoulders to the hem, and a hood sitting on top. The hood is not
solid — it has a genuine cut-out aperture (the drawstring opening,
the one shape a hood is best known by), and inside that opening,
seen through it, sit two ascending columns of a bar chart. The
opening is doing two jobs with one shape: it is the hood's face-hole
and the chart's plot area at the same time, not a coat with a chart
drawn on next to it.

That's the whole brief in one mark: "gilet" as the obsessive-hobbyist
slang (the data glimpsed through the hood — someone who'd notice a
player's hidden Current Ability), and "gilet" as the literal
garment — the joke being that an gilet is a kind of mac(k), and the
app is a Mac (and Windows) app.

## Colour

- `#ff6a13` (hi-vis orange, Accent 1) is the garment. It leads,
  because orange is the technical-wear colour and "user intent" is
  the app's own reading of it.
- `#3fbfa8` (teal, Accent 2) is only the two columns inside the hood
  opening — the one place the data colour appears, deliberately where
  you'd be looking if you peered into the hood.
- `#0e1419` / `#161e26` / `#263340` are the app's background, panel,
  and border, used as-is for the icon's backing plate in `logo.svg`
  (full-bleed background + an inset panel with a small radius and a
  hairline border, matching the app's own chrome).

The aperture is a true cut-out (`fill-rule: evenodd`), not a
same-colour rectangle painted over the hood, so it survives as real
negative space at any single colour or on any background — check it
with the colour stripped out, it should still read as a hood with an
opening, not just a solid badge.

## Files

- `logo.svg` — square mark, `viewBox 0 0 512 512`, with its own dark
  backing plate. This is the source for the app icon.
- `logo-wordmark.svg` — the same mark (bare, no backing plate, true
  transparency) set beside "Gilet" in IBM Plex Sans Condensed Bold,
  for use directly on the app's dark background (headers, about box,
  etc). The project already ships `@fontsource/ibm-plex-sans-condensed`,
  so the font is available inside the app; the `font-family` also
  lists system condensed-sans fallbacks for anywhere else it's
  opened. For any export where font availability isn't guaranteed
  (marketing assets, print, a design tool that won't have the
  fontsource package), convert the `<text>` element to outlines
  (e.g. Illustrator/Inkscape "Object to Path", or Figma "Outline
  Stroke"/"Flatten") and drop the `font-family` — the shapes will
  then render correctly with zero font dependency.

## Regenerating the app icon set

From the repo root, with the Tauri CLI already a dev dependency:

```
bunx tauri icon assets/logo.svg
```

`tauri icon` accepts an SVG source directly and writes the full
platform icon set to `src-tauri/icons` (next to `tauri.conf.json`) by
default. Pass `-o <dir>` to write somewhere else. If you ever need a
flat PNG source instead (some downstream tool that won't take SVG),
rasterize `logo.svg` at 1024x1024 first and pass that path instead.
