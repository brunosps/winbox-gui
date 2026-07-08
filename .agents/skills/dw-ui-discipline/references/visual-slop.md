# Visual slop — 14 patterns + 17 default values to avoid

Two parts:
1. **Fourteen patterns** an ungrounded UI agent produces.
2. **Seventeen specific values** that signal "no thought went into this."

Used by `/dw-review --code-only` against UI diffs and by `/dw-redesign-ui` as a self-check during proposal.

## The 14 patterns

### 1. Uniform-section flatness

Every section uses the same card style, same padding, same text size, same emphasis weight. The eye finds no anchor.

- **Why it happens:** Default of "consistent = good" without realizing hierarchy needs deliberate variation.
- **Fix:** One primary section per scroll height. Differentiate by size, weight, color saturation, or whitespace by ≥30%. Everything else recedes.
- **Example violation:** Dashboard with 6 identical metric cards.
- **Example fix:** One hero metric (largest, top); 3 supporting metrics; 2 minor metrics in a different visual treatment.

### 2. Soft hierarchy

Headings barely larger than body. Primary CTA same color as secondary. The user can't tell what to look at first.

- **Why it happens:** "Elegant restraint" applied without ensuring guidance still works.
- **Fix:** Squint at the design (literally). What jumps out? If nothing jumps out, increase contrast in size, weight, or color for the primary element.

### 3. Decorative hover

Hover effects on elements that have no click handler. Cards that fade slightly but don't link anywhere.

- **Why it happens:** Default "apply hover to anything card-shaped."
- **Fix:** Hover effect lives only on elements with an on-click. Non-interactive shapes get `cursor: default`. If it's hoverable, it must do something.

### 4. Emoji as ornament

Emojis in headers and section labels where they add no information: 🎯 Goals · 🚀 Launch · ✨ Features · 📊 Analytics · 🔥 Trending.

- **Why it happens:** Training data has many "emoji-in-headers = engaging" patterns.
- **Fix:** Use icons (lucide, heroicons, tabler) for semantic meaning. Reserve emojis for genuinely emotive contexts (celebrations, errors needing empathy). If removing the emoji preserves the meaning, remove it.

### 5. Gradient cover

Hero with diagonal purple-to-pink gradient. Buttons with subtle gradient. Card backgrounds with mesh gradients. Gradient as visual fallback for weak composition.

- **Why it happens:** AI-art aesthetics leak into UI; gradients hide compositional weakness.
- **Fix:** A gradient must earn its place — usually for hero zones with poetic copy. Solid colors with strong hierarchy beat gradients in utility surfaces.

### 6. Glass-on-everything

Frosted-glass effect on modals, cards, dropdowns, side panels — anywhere a surface can be layered. Including on top of plain backgrounds where the blur effect has nothing to blur.

- **Why it happens:** macOS aesthetic. Looks premium without effort.
- **Fix:** Glass only when there's meaningful content visible behind the surface. Glass over plain backgrounds adds visual noise without semantic gain.

### 7. Center-aligned by default

Body paragraphs center-aligned. Headlines centered. Forms with labels centered above inputs. Tabular data centered instead of column-aligned.

- **Why it happens:** Marketing-page training data biases toward center.
- **Fix:** Center for hero headlines and small CTA labels only. Body text and forms read better left-aligned in LTR scripts. Tabular data reads in columns.

### 8. Grayscale wash

Neutral gray palette everywhere — `slate-50`, `gray-100`, `zinc-200` — for backgrounds, borders, text, accents. No accent color, no character.

- **Why it happens:** "Neutral = safe" plus shadcn/ui's neutral starting point.
- **Fix:** Establish ONE accent color (from brand or curated defaults). Use it intentionally on the primary CTA, the active state, the one place the user looks first. Gray is the canvas, not the painting.

### 9. Verb-less CTAs

"Get Started" · "Learn More" · "Click Here" · "Submit" · "OK" buttons. Generic verbs that say nothing.

- **Why it happens:** Default LLM verb library.
- **Fix:** Use the verb the user is actually doing. "Approve refund" not "Submit". "Start free trial" not "Get Started". "Schedule a call" not "Contact us".

### 10. Stock-illustration hero

Figure-with-laptop hero art. Diverse-team-around-table illustration. Abstract floating shapes. Generic figures from illustration kits.

- **Why it happens:** "Illustration = friendly" default. Cheap to produce.
- **Fix:** Use product screenshots (real screens, real data, sanitized) or skip illustration entirely. A clean hero with strong typography beats generic illustration.

### 11. Shadow soup

Cards with shadow. Buttons with shadow. Inputs with shadow. Tooltips with shadow on shadows. Borders AND shadows AND gradients on one element.

- **Why it happens:** Material Design leftover; depth as decoration.
- **Fix:** Pick one depth mechanism per layer. If cards have shadow, buttons inside should not. If you use elevation systematically (Material 3), enforce the elevation hierarchy.

### 12. Generic spinner

Spinner overlay for every async operation, regardless of duration or context.

- **Why it happens:** Default fallback in every UI library.
- **Fix granularity:**
  - <300ms: show nothing (spinner appearing then vanishing is flicker).
  - 300ms–2s: skeleton loader matching content shape.
  - 2s–10s: spinner + status text ("Loading orders...").
  - 10s+: progress bar or step indicator + cancel button.

### 13. Silent empty state

"No items found." Centered. Nothing else. User has no idea what to do.

- **Why it happens:** Empty state treated as edge case, not as a real screen.
- **Fix:** Every empty state answers two questions: WHY is it empty (no data yet vs filter excluded everything vs error)? WHAT should the user do (CTA, like "Create your first invoice")?

### 14. Toast spam

Every UI event becomes a toast. Save successful → toast. Validation error → toast. Network slow → toast. Five stack up and the user reads none.

- **Why it happens:** Toast is the default feedback mechanism in component libraries.
- **Fix:** Toasts only for actions that need confirmation AWAY from the originating surface (background save, undo-able deletion). Inline feedback for form validation. Modal/banner for blocking errors. Cap at 2 stacked toasts.

## The 17 anti-default values

Specific values that signal "no thought went into this." Avoid unless you can articulate WHY you picked exactly this one:

| Anti-default | Tell |
|--------------|------|
| `#3B82F6` (Tailwind blue-500) | The internet's default blue |
| `rounded-lg` everywhere | Universal default; no surface character |
| `shadow-md` on every card | Universal default; no depth hierarchy |
| `bg-gradient-to-br from-purple-500 to-pink-500` | "AI startup landing page" gradient |
| Inter as the only font choice | Default font of ~60% of new SaaS |
| `font-bold` for every emphasis | Bold is one tool, not the only tool |
| Lucide icons exclusively | One icon family is fine; signature is none |
| Generic "happy team" hero illustration | Placeholder energy |
| "Get Started" / "Learn More" CTA copy | Verb-less; says nothing |
| 4 / 8 / 12 / 16 spacing exclusively | The default 4-step scale; no rhythm |
| `border-gray-200` for every divider | Visual whisper; no intent |
| Sans-serif headlines + sans-serif body | No typographic contrast |
| Center-aligned everything | See pattern #7 |
| Animated CSS confetti on success | Cheesy; mismatches most brands |
| `bg-white dark:bg-gray-900` only | No real dark-mode design pass |
| Single-column form on a wide screen | Vertical scroll where horizontal fits |
| Modal for every interaction | Most modals should be inline editing |

## How to apply this catalog

In `/dw-redesign-ui` step 4 (propose) — before presenting design directions, self-check against this list. If you're using a pattern, say why explicitly ("gradient crutch — intentional for marketing hero"). Sometimes the pattern IS the right call; the discipline is awareness, not absolutism.

In `/dw-review --code-only` UI section — grep the diff for the anti-default values and the patterns. Each hit becomes a finding under `dw-review-rigor`:
- Pattern on a NEW surface → `medium` severity.
- Pattern propagating EXISTING slop further → `low` severity (consistency wins).
- Pattern on a redesign that was supposed to fix slop → `high` severity (regression).

## When the patterns bend

- **Marketing pages** can use gradients and emojis with more freedom — different surface job.
- **Brand-mandated** values override this list (if your brand IS `#3B82F6`, use it).
- **Component libraries** like shadcn ship neutral defaults — the discipline is to ADD character on top, not remove the neutrality.
