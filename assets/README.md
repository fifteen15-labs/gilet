# Gilet mark

`logo.svg` is a literal icon of a gilet — the sleeveless padded
body-warmer every football manager and scout wears pitchside. It is
drawn flat and front-on, like a kit supplier's product glyph, not an
abstract badge.

Three garment features do the work, in order of importance:

1. **Armholes.** A deep scoop is cut into each side at the shoulder,
   so the shoulders flare out and the torso cinches in below them
   before squaring off again at the hip. This is the one feature that
   says "sleeveless" and rules out a jacket or t-shirt, so it is drawn
   big and bold — small or fussy armholes are exactly the kind of
   detail that turns to mush at icon size, so this one doesn't stay
   small.
2. **A centre-front zip** — a single vertical slit from the base of
   the collar to the hem, splitting the garment into two panels.
3. **A stand-up collar** — the top edge is two flat-topped points
   either side of a centre V, rather than a plain flat neckline. This
   is what pushes the read toward "coach on the touchline" rather than
   "generic hi-vis vest."

A fourth, lower-priority detail — three rows of horizontal quilted
baffle seams — adds the padded, quilted texture a real gilet has. It
was checked at 32px alongside everything else and survives, so it
stays; if it had turned to mush it would have been cut, per the same
rule applied to the armholes.

## Why "gilet"

The name is a football joke, not a wordplay stretch: a gilet is
specifically the touchline-manager garment — what every manager and
member of the coaching staff is wearing in the dugout in February. It
says "I am the one watching, assessing, taking notes," not "I am
playing." That's exactly what this tool does with a save file: watch,
assess, take notes on players the game itself doesn't surface. The
mark plays that completely straight — it's a gilet, unmistakably,
with nothing else layered into the shape.

## Colour

- `#ff6a13` (hi-vis orange, Accent 1) is the only colour on the
  garment. Orange is the technical-wear colour, and one confident
  colour on a dark ground reads like a kit-supplier product mark
  rather than a decorative illustration.
- `#3fbfa8` (teal, Accent 2) is **not** used on the mark. The brief
  keeps it in reserve for data surfaces elsewhere in the app (charts,
  filters, badges) — it didn't earn a place on an icon this simple.
- `#0e1419` is the background in `logo.svg` (full-bleed, no inset
  panel — kept plain since the mark is the only thing that needs to
  read here).
- `#e8eff4` is the wordmark text colour in `logo-wordmark.svg`.

The garment's cutouts — the two armhole scoops, the centre zip, and
the six quilting-seam segments — are constructed as genuine negative
space (the armholes are notches cut into the outer contour; the zip
and quilting are interior holes via `fill-rule: evenodd`), not a
second colour painted on top. That's what makes the mark survive
being re-filled as a single flat colour: strip the orange out
entirely and the cutouts still show through as real gaps, so the
silhouette alone — armholes, collar, zip, quilting — still reads as
"gilet" with no colour and no context.

## Checks

Both hard requirements were verified by actually rendering the SVGs
to PNG (`rsvg-convert` + ImageMagick), not just eyeballing the source:

- **32px.** Rendered at 32×32 and inspected at that size (and
  upscaled with a smooth filter to see it clearly): the armhole
  scoops, collar points, centre zip and quilting rows are all still
  legible. Nothing is riding on detail that only exists at 512px.
- **Monochrome silhouette.** Re-filled to a single flat colour (no
  orange, no background colour distinction) and re-rendered at 32px:
  the shape alone — cutouts included, since they're true holes rather
  than a colour trick — still reads unmistakably as a sleeveless,
  zip-front, collared vest.

Two earlier directions were tried and dropped along the way:

- A version that fused a data motif (a bar chart hidden in the
  quilting, or the zip doubling as a chart axis) into the garment
  shape. That was a deliberate brief from an earlier pass, but the
  product owner cut it: the ask is a plain, literal gilet icon, not a
  garment-plus-data pun. Baking in a second meaning also worked
  against the 32px requirement — the more a single shape is asked to
  carry, the more likely some part of it mushes at small size.
- A rounder variant with softer, semicircular armholes and blockier
  "ear"-like collar tabs. It held up fine at 32px too, but the
  pointed collar in the shipped version reads more clearly as a
  *collar* (structured lapel points) rather than something closer to
  ears or a crown, so it was preferred.

## Files

- `logo.svg` — square mark, `viewBox 0 0 512 512`, with its own dark
  background. This is the source for the app icon.
- `logo-wordmark.svg` — the same mark (bare, no background, true
  transparency) set beside "Gilet" in IBM Plex Sans Condensed Bold,
  for use directly on the app's dark background (headers, about box,
  etc). The project already ships
  `@fontsource/ibm-plex-sans-condensed`, so the font is available
  inside the app; the `font-family` also lists system condensed-sans
  fallbacks for anywhere else it's opened. For any export where font
  availability isn't guaranteed (marketing assets, print, a design
  tool that won't have the fontsource package), convert the `<text>`
  element to outlines (e.g. Illustrator/Inkscape "Object to Path", or
  Figma "Outline Stroke"/"Flatten") and drop the `font-family` — the
  shapes will then render correctly with zero font dependency.

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
