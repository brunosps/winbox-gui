# Template Overrides

Files in this directory override the corresponding templates in `.dw/templates/`. Use this when your team needs a customized PRD/TechSpec/Task/etc. shape **without** losing the ability to receive `dev-workflow` updates.

## How it works

- During `dev-workflow init` or `dev-workflow update`, the CLI copies templates from the package into `.dw/templates/`.
- Before writing each file, the CLI checks `.dw/templates/overrides/` for a same-named file.
- If found, **the override is preserved and the core template is skipped**. Your customization wins.
- If not found, the core template is installed/updated normally.

## How to override a template

```bash
# 1. Copy the template you want to customize from .dw/templates/ to .dw/templates/overrides/
cp .dw/templates/prd-template.md .dw/templates/overrides/prd-template.md

# 2. Edit the override to fit your team's process
$EDITOR .dw/templates/overrides/prd-template.md

# 3. Commit it. Future updates won't touch this file.
git add .dw/templates/overrides/prd-template.md
git commit -m "chore(templates): customize PRD template for our team"
```

The override takes effect immediately. Commands that consume the template (`/dw-plan prd`, etc.) will read from `.dw/templates/<name>.md`, which is preserved as your edited copy.

## How to revert an override

```bash
# 1. Delete the override
rm .dw/templates/overrides/prd-template.md

# 2. Re-run dev-workflow update to restore the core template
npx @brunosps00/dev-workflow update
```

## When an override is appropriate

- Adding required sections specific to your industry (compliance, finance, healthcare).
- Removing sections that don't apply.
- Renaming sections to match your team's vocabulary.
- Embedding links to internal tools, dashboards, runbooks.

## When an override is **not** appropriate

- Removing critical-path sections like FR numbering or test plans — downstream commands rely on these.
- Adding fields that conflict with the YAML frontmatter schema.
- Anything that breaks `/dw-plan techspec` or `/dw-plan tasks` cross-references.

If in doubt, run the workflow end-to-end after editing — the consistency check at the end of `/dw-plan tasks` will surface most structural breakages.

## Versioning your overrides

When `dev-workflow` releases a new minor version, the core templates may change shape (new sections, renamed fields). Your override will continue to work, but it might lack the new sections.

Recommended cadence: when you upgrade `dev-workflow`, `diff` your override against the new core template:

```bash
diff .dw/templates/overrides/prd-template.md .dw/templates/prd-template.md
```

Merge in anything you want from upstream. The override stays canonical; the core template is just a reference.

## Subdirectories

Overrides can mirror the directory structure of `.dw/templates/`. For example, to override a file inside `functional-doc/`:

```
.dw/templates/overrides/
└── functional-doc/
    └── e2e-runbook.md
```

The resolver descends recursively — each leaf file is checked independently against its corresponding path under `overrides/`.
