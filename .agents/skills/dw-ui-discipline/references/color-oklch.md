# Color & contrast — OKLCH, tinted neutrals, dark mode

> Patterns adapted from [impeccable](https://github.com/pbakaus/impeccable) (Apache-2.0, derived from
> Anthropic's frontend-design skill). Read when building or reviewing a palette.

## Work in OKLCH, not hex/HSL

OKLCH (`oklch(L C H)`) is perceptually uniform: equal lightness steps *look* equally spaced, and changing
hue does not shift perceived brightness (HSL's big flaw — HSL yellow at 50% L looks far lighter than HSL
blue at 50% L). Modern browsers support `oklch()` natively.

- **L** (lightness) 0–100% — drive your scale off this.
- **C** (chroma) 0–~0.37 — saturation. Keep neutrals low (0.005–0.03), accents moderate (0.1–0.2).
- **H** (hue) 0–360.

## Tinted neutrals (the single highest-leverage move)

Pure grays (`#777`, `oklch(50% 0 0)`) look dead. Tint every neutral slightly toward one hue — usually the
brand hue or its complement — so the UI feels intentional.

```css
/* Cool, slightly blue-tinted neutral scale */
--n-50:  oklch(98% 0.005 250);
--n-100: oklch(96% 0.006 250);
--n-300: oklch(87% 0.010 250);
--n-500: oklch(62% 0.014 250);
--n-700: oklch(42% 0.014 250);
--n-900: oklch(22% 0.012 250);
--n-950: oklch(14% 0.010 250);
```

Pure `#000` / `#fff` are anti-patterns (the detector flags `pure-black-white`). Use `oklch(14% 0.01 H)`
for "black" surfaces and `oklch(98% 0.005 H)` for "white".

## Building a scale

Pick the hue + chroma, then step lightness on a roughly even curve (98 / 96 / 92 / 87 / 78 / 68 / 62 / 52 /
42 / 32 / 22 / 14). Keep chroma roughly constant for neutrals; for an accent, let chroma peak in the mid
lightness range (most vivid at 55–65% L) and fall off at the extremes.

## Accent & semantic colors

- One primary accent carries the brand. Avoid the training-data default blue `#3B82F6` — shift the hue or
  darken it (`oklch(55% 0.18 255)` reads more deliberate).
- Semantic set: success (green ~145H), warning (amber ~75H), danger (red ~25H), info (blue ~250H). Keep
  their lightness consistent with each other so badges feel like a family.

## Contrast (non-negotiable, see accessibility-floor.md)

- Body text ≥ 4.5:1; large text (≥24px or ≥18.7px bold) and UI component boundaries ≥ 3:1.
- Don't rely on hue alone to convey state — pair color with icon/text (color-blind users).
- Test the *actual* foreground/background pair, not the token names.

## Dark mode

Dark mode is not "invert lightness". Re-tune:
- Raise surface lightness in steps as elements come forward (`oklch(16%)` page → `18%` card → `22%`
  popover) instead of borders everywhere.
- Reduce accent chroma slightly in dark mode — vivid colors vibrate on dark backgrounds.
- Keep body text around `oklch(90% 0.01 H)`, not pure white, to cut glare.
