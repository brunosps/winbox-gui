# Typography — modular scale, pairing, measure

> Patterns adapted from [impeccable](https://github.com/pbakaus/impeccable) (Apache-2.0, derived from
> Anthropic's frontend-design skill). Read when choosing fonts or fixing hierarchy.

## Modular scale

Pick a base size (16px body) and a ratio, then derive every step from it — don't hand-pick arbitrary
sizes. Common ratios: 1.2 (minor third, dense UI), 1.25 (major third, balanced), 1.333 (perfect fourth,
editorial/marketing).

Example at base 16 / ratio 1.25:
`12 · 14 · 16 · 20 · 25 · 31 · 39 · 49` (px). Map to roles: caption 12–14, body 16, h4 20, h3 25, h2 31,
h1 39–49. **Soft hierarchy** (headings barely larger than body) is a visual-slop pattern — make the jump
visible.

## Fluid type

For headings that should scale with viewport, use `clamp()` instead of breakpoint steps:

```css
h1 { font-size: clamp(1.75rem, 1.2rem + 2.5vw, 3rem); }
```

Keep body text fixed (≥16px) — fluid body harms readability and zoom.

## Pairing

- One typeface, multiple weights, is the safest path — pairs fail more often than they succeed.
- If pairing: contrast roles clearly (e.g., a distinctive display face for headings + a neutral grotesque
  for body). Don't pair two similar sans fonts.
- Avoid the overused defaults that signal "no thought": Inter-everywhere, system-ui only, Montserrat
  headings, Lobster/Pacifico for "personality". A deliberate grotesque or a good serif body reads better.

## Weight, line-height, measure

- **Weight:** body 400–450; don't set body below 400 (thin fonts fail in bright light — see the scene
  question). Headings 600–700. Avoid 300 for anything users must read.
- **Line-height:** body ~1.5–1.6; headings tighter ~1.1–1.25; the larger the text, the tighter.
- **Measure (line length):** 45–75 characters for body. Cap with `max-width: 65ch`. Over ~80ch the eye
  loses the next line — the detector flags long lines.

## Letter-spacing & numerics

- Tighten large headings slightly (`letter-spacing: -0.01em` to `-0.02em`); leave body alone.
- All-caps labels need positive tracking (`0.05em`).
- Use tabular figures (`font-variant-numeric: tabular-nums`) for tables, timers, prices so digits don't
  jitter.
