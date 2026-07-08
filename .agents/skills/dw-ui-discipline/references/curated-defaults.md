# Curated defaults — 10 palettes + 10 font pairings + spacing scale

Use this reference when the project has **no design authority** (no `.dw/rules/` design section, no `DESIGN.md`, no Tailwind theme tokens, no component library config).

The values below cover ~90% of cases that need a starting point. Not opinionated branding — neutral, accessible, professional defaults.

## How to use

1. Pick ONE palette + ONE font pairing + the spacing scale.
2. Commit the choice to the project (`.dw/rules/<frontend>.md` design section OR a new `DESIGN.md`).
3. From there, the project HAS a design authority; downstream `dw-ui-discipline` invocations defer to that authority.

This is bootstrap, not destination. The goal is to escape "no authority" mode within the first feature.

## The 10 palettes

Each palette below: neutral scale (gray, slate, etc.) + 1 accent + semantic colors. All WCAG 2.2 AA compliant for body text.

### 1. Cool Stone
- Neutrals: Tailwind `slate` (50-950)
- Accent: `#3B82F6` (blue-500) — wait, no, see Anti-Slop. Use `#0066CC` instead.
- Semantic: success `#16A34A`, warning `#D97706`, danger `#DC2626`
- **Tone:** Calm, corporate, trustworthy. Default for SaaS dashboards.

### 2. Warm Sand
- Neutrals: Tailwind `stone` (50-950)
- Accent: `#CA8A04` (warm amber)
- Semantic: success `#16A34A`, warning `#EA580C`, danger `#DC2626`
- **Tone:** Approachable, hospitality-adjacent, organic.

### 3. Forest Pine
- Neutrals: Tailwind `gray` (50-950)
- Accent: `#15803D` (deep green)
- Semantic: success `#16A34A`, warning `#D97706`, danger `#B91C1C`
- **Tone:** Steady, financial, sustainable.

### 4. Plum Velvet
- Neutrals: Tailwind `zinc` (50-950)
- Accent: `#7C3AED` (violet-600)
- Semantic: success `#22C55E`, warning `#EAB308`, danger `#E11D48`
- **Tone:** Creative, premium, design-tool.

### 5. Coral Reef
- Neutrals: Tailwind `neutral` (50-950)
- Accent: `#E11D48` (rose-600)
- Semantic: success `#16A34A`, warning `#F59E0B`, danger `#B91C1C`
- **Tone:** Energetic, consumer, retail.

### 6. Steel Blue
- Neutrals: Tailwind `slate` (50-950)
- Accent: `#0F766E` (teal-700)
- Semantic: success `#16A34A`, warning `#D97706`, danger `#DC2626`
- **Tone:** Technical, infrastructure, devops.

### 7. Burnt Sienna
- Neutrals: Tailwind `stone` (50-950)
- Accent: `#C2410C` (orange-700)
- Semantic: success `#15803D`, warning `#A16207`, danger `#B91C1C`
- **Tone:** Bold, news, sports.

### 8. Ink and Paper
- Neutrals: Pure `#000` and `#FFF` with `#E5E5E5` divider
- Accent: `#000` (no accent — minimal)
- Semantic: success `#16A34A`, warning `#D97706`, danger `#DC2626`
- **Tone:** Editorial, brutalist, magazine. Pair with serif body.

### 9. Midnight Indigo
- Neutrals: Tailwind `slate` dark-mode-first (900-50)
- Accent: `#6366F1` (indigo-500)
- Semantic: success `#22C55E`, warning `#FACC15`, danger `#EF4444`
- **Tone:** Tech-forward, dark-mode-default, on-call tooling.

### 10. Sea Salt
- Neutrals: Tailwind `gray` cool (50-950)
- Accent: `#0891B2` (cyan-600)
- Semantic: success `#10B981`, warning `#F59E0B`, danger `#F43F5E`
- **Tone:** Fresh, healthcare, education.

## The 10 font pairings

Each pairing: heading font / body font. All available on Google Fonts. Variable fonts where possible.

| # | Headings | Body | Tone |
|---|----------|------|------|
| 1 | Inter | Inter | Modern SaaS default. Workhorse pairing. |
| 2 | Geist Sans | Geist Sans | Vercel-aesthetic; clean, geometric. |
| 3 | Söhne (or Manrope) | Söhne (or Manrope) | Editorial-modern; replaces Inter when Inter feels overused. |
| 4 | DM Serif Display | Inter | Editorial-meets-product. Strong headlines, neutral body. |
| 5 | Fraunces | Inter | Variable serif headlines, sans body. Premium feel. |
| 6 | Source Serif 4 | Source Sans 3 | Adobe pairing; balanced editorial. |
| 7 | Space Grotesk | Inter | Slightly quirky headlines, neutral body. Tech with personality. |
| 8 | IBM Plex Serif | IBM Plex Sans | Industrial, opinionated. Good for enterprise tooling. |
| 9 | Playfair Display | Manrope | High contrast pairing; hospitality, lifestyle. |
| 10 | JetBrains Mono | Inter | Code-first products; mono for headlines and accents. |

**Anti-default reminder:** Inter / Inter is fine. Inter / Inter as the ONLY pairing you ever pick is a tell.

## Spacing scale

Use a 4px-base scale with intentional jumps:

```
0:   0px
1:   4px   (tight controls, icon padding)
2:   8px   (compact spacing)
3:   12px  (default gap in form fields)
4:   16px  (default gap between blocks)
6:   24px  (section padding)
8:   32px  (large section spacing)
12:  48px  (page header / hero padding)
16:  64px  (between major sections)
24:  96px  (hero / landing block separators)
32:  128px (rare; signature spacing)
```

**Skip these by default:** 5, 7, 9, 10, 11, 13, 14, 15. Picking exactly 4-6-8-12-16 reads more intentional than 1-2-3-4-5-6-...

## Border radius scale

Pick a "personality" once and use it consistently:

| Radius | Tone |
|--------|------|
| 0 (square) | Brutalist, editorial |
| 2px | Subtle modernist |
| 4px (Tailwind `rounded`) | Standard |
| 6px | Slightly friendly |
| 8px (Tailwind `rounded-lg`) | Default SaaS; **anti-default if used universally** |
| 12px | Softer, consumer |
| 16px+ | Rounded everything; friendly/youth-oriented |
| Full / 9999px | Pills, buttons, avatars only |

Don't mix more than 2 radii in one surface (e.g., 4px for inputs, 8px for cards). Three+ radii = visual noise.

## Type scale

For body=16px baseline:

```
xs:    12px  (captions, helper text — minimum 11px)
sm:    14px  (secondary, labels)
base:  16px  (body)
lg:    18px  (lead paragraphs)
xl:    20px  (small headings)
2xl:   24px  (subheadings)
3xl:   30px  (section headings)
4xl:   36px  (page titles)
5xl:   48px  (hero, marketing)
6xl+:  60-96px (display, marketing only)
```

Line-height pairs:
- Body text: 1.5 to 1.6
- Headings: 1.1 to 1.25
- Buttons: 1 (tight)

## How to commit a choice

After picking a palette + pairing + spacing, write `.dw/rules/<frontend>.md` (or `DESIGN.md`) with:

```markdown
## Design System (bootstrap)

**Source:** Curated default — `dw-ui-discipline/references/curated-defaults.md`, picked Cool Stone + Inter/Inter + 4px-base scale on 2026-05-12.

### Colors
- Neutrals: Tailwind slate (50-950)
- Accent: #0066CC (links, primary buttons, focus rings)
- Success: #16A34A · Warning: #D97706 · Danger: #DC2626

### Typography
- Headings: Inter, weight 600-700
- Body: Inter, weight 400-500
- Mono: JetBrains Mono (code blocks only)

### Spacing
4px base scale: 4 / 8 / 12 / 16 / 24 / 32 / 48 / 64 / 96.

### Border radius
4px on inputs, 8px on cards, full on pills/avatars.

### Updates
This file is the canonical design source. To change a value, propose a PR
that updates this file FIRST, then propagates through code.
```

Once written, `dw-ui-discipline` reads this file instead of `curated-defaults.md`. The bootstrap step is done.

## What this catalog is NOT

- Not 161 palettes. Not 57 fonts. By design.
- Not a substitute for brand work — for a serious product, hire a designer.
- Not opinionated about industry-specific aesthetics (med-tech, gaming, kids' content). Those need real design pass.

The point is: have ONE intentional starting point so the agent isn't reaching for `#3B82F6` and `rounded-lg`. Project-specific design follows.
