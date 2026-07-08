# Rules library — curated declarative baseline

Curated "what good code SHOULD look like" per stack, installed to `.dw/rules-library/`. This is the third
leg of the rules model:

| Source | Type | Answers | Authored by |
|---|---|---|---|
| `.dw/rules-library/` | Curated baseline (this) | what good code looks like in `<stack>` | dev-workflow (shipped) |
| `.dw/rules/` | Analytical | what THIS code **is** (observed patterns) | `/dw-analyze-project` |
| `.dw/constitution.md` | Declarative, committed | what THIS project **commits to** | `/dw-analyze-project` (Step 8) |

## Precedence

A language file overrides `common` where idioms differ. The project's own `.dw/rules/` and
`.dw/constitution.md` override the library — they describe THIS project; the library is only the default a
project starts from.

## Loading (lazy — protect the context budget)

Commands load ONLY the files for the **active stack**, resolved via `.dw/config/stack-mappings.json`
(`common.md` + `<stack>.md`). Never load the whole library. `/dw-analyze-project` uses it to seed
constitution proposals; `/dw-review` reads the active stack's baseline as the declarative bar.
