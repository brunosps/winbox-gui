<system_instructions>
You are an update utility. When invoked, update dev-workflow to the latest version published on npm without requiring the user to leave the agent.

## When to Use
- Use when the user wants to update `/dw-*` commands, templates, references, scripts, skills, wrappers, and MCPs to the latest version
- Use when a new release is out and the user wants to apply it without exiting the session
- Do NOT use to install from scratch in a new project (use `npx dev-workflow init`)
- Do NOT use to install system dependencies/Playwright/MCPs (use `npx dev-workflow install-deps`)

## Pipeline Position
**Predecessor:** (any) | **Successor:** (any)

## Modes

- **Update (default)**: `/dw-update` — updates to the latest version on npm
- **Rollback**: `/dw-update --rollback` — restores the most recent snapshot in `.dw/.backup/` (created before each update)

## Behavior

### 0. Snapshot Before Update (Required in default mode)

Before overwriting managed files, create a snapshot:

```bash
SNAPSHOT_DIR=".dw/.backup/$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$SNAPSHOT_DIR"
cp -r .dw/commands .dw/templates .dw/references .dw/scripts "$SNAPSHOT_DIR/" 2>/dev/null
[ -d .agents/skills ] && cp -r .agents/skills "$SNAPSHOT_DIR/agents-skills" 2>/dev/null
echo "Snapshot saved to $SNAPSHOT_DIR"
```

Keep only the 3 most recent snapshots (remove older ones) to avoid buildup.

### 1. Record Current Version (Required)

Before updating, capture the installed version so you can report the delta:

```bash
node -e "try { console.log(require('@brunosps00/dev-workflow/package.json').version) } catch(e) { console.log('not-cached-locally') }" 2>/dev/null
```

### 2. Detect Language of Installed Commands (Required)

<critical>Do NOT assume the language from this command file. Detect by inspecting the actual files in `.dw/commands/` so a user with a mixed or switched install does not receive an update in the wrong language.</critical>

Run at the project root:

```bash
if [ -d .dw/commands ]; then
  PT_COUNT=$(grep -l "## Quando Usar" .dw/commands/*.md 2>/dev/null | wc -l)
  EN_COUNT=$(grep -l "## When to Use" .dw/commands/*.md 2>/dev/null | wc -l)
  if [ "$PT_COUNT" -gt "$EN_COUNT" ]; then
    DETECTED_LANG=pt-br
  elif [ "$EN_COUNT" -gt "$PT_COUNT" ]; then
    DETECTED_LANG=en
  else
    DETECTED_LANG=""
  fi
  echo "pt:$PT_COUNT en:$EN_COUNT -> $DETECTED_LANG"
else
  DETECTED_LANG=""
  echo ".dw/commands does not exist"
fi
```

Rules:
- If `DETECTED_LANG` is `pt-br` or `en`: use it in the next step
- If empty (tie, missing folder, new install): ask the user `I detected [describe context]. Proceed with pt-br or en?` and wait for the response before continuing

### 3. Run the Update (Required)

<critical>Use `npx -y @brunosps00/dev-workflow@latest` to FORCE fetching the latest version from npm (bypass local cache). Pass `--lang=<DETECTED_LANG>` to skip the interactive prompt.</critical>

```bash
npx -y @brunosps00/dev-workflow@latest update --lang=$DETECTED_LANG
```

The `update` command overwrites managed files and PRESERVES:
- `.dw/rules/` (user rules)
- `.dw/spec/` (in-progress PRDs and tasks)
- `.planning/` (user data)

If the update fails (network error, permission, package unavailable): report the error to the user and STOP. Do NOT attempt manual workarounds like copying files by hand.

### 4. Capture New Version

```bash
node -e "console.log(require('@brunosps00/dev-workflow/package.json').version)" 2>/dev/null
```

### 5. Report Result

Present to the user:
- Detected language (`DETECTED_LANG`)
- Previous version → new version
- Summary of what the `update` output showed (files copied, wrappers generated, MCPs configured)
- Any warnings or errors

### 6. Suggest Next Step

If commands/skills were updated, remind the user:
- Restart the agent session (or reload skills) so the new instructions take effect — skills are usually loaded at session start
- Run `/dw-help` after the reload to see the updated command set
- If the release changed system dependencies (Playwright, MCPs), run `npx dev-workflow install-deps` separately

## Rollback Mode

If invoked with `--rollback`:

1. List snapshots in `.dw/.backup/`
2. If none exist: STOP and report "No snapshot available"
3. If more than one exists: ask the user which to restore (default: most recent)
4. Confirm with the user: "Restore snapshot `<path>`? This OVERWRITES `.dw/commands/`, `.dw/templates/`, `.dw/references/`, `.dw/scripts/`, and `.agents/skills/`. Proceed? [y/N]"
5. Only after `y`: copy back

```bash
cp -r "$SNAPSHOT_DIR/commands"   .dw/
cp -r "$SNAPSHOT_DIR/templates"  .dw/
cp -r "$SNAPSHOT_DIR/references" .dw/ 2>/dev/null
cp -r "$SNAPSHOT_DIR/scripts"    .dw/ 2>/dev/null
[ -d "$SNAPSHOT_DIR/agents-skills" ] && cp -r "$SNAPSHOT_DIR/agents-skills" .agents/skills 2>/dev/null
```

6. Report: snapshot restored, version likely recovered (read from `.dw/commands/dw-help.md` or metadata if present)

## Advanced Options

If the user asks for a specific version (not `@latest`):

```bash
npx -y @brunosps00/dev-workflow@<version> update --lang=$DETECTED_LANG
```

E.g.: `npx -y @brunosps00/dev-workflow@0.4.5 update --lang=en`

## Notes

- `npx -y` skips the "OK to install" prompt when the package is not cached
- `@latest` bypasses the npx cache and pulls the `latest` tag from the registry
- `--lang=...` skips the interactive language prompt; the value comes from the auto-detection in step 2
- This command does NOT update the user project's Node dependencies — only the dev-workflow scaffold

</system_instructions>
