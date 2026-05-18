# Incremental update — keeping `.dw/intel/` fresh without re-scanning everything

A full scan of a 50K-line repo takes minutes and burns context. Incremental updates target only what changed, in seconds.

## When to run a full update

- First analysis (no `.dw/intel/` yet)
- After major restructuring (migration, framework change, large refactor)
- After 30+ days since last refresh (file structure has likely drifted)
- When `/dw-intel` queries return obviously stale results

Trigger via `/dw-intel --build` (no flag) or `/dw-intel --build --full`.

## When to run a partial update

- A single PR / feature branch touched 1-20 files
- After `/dw-run` completes (touched files are known via git)
- After `dw-deps-audit --execute` updates dependencies (only `deps.json` needs refresh)

Trigger via `/dw-intel --build --files src/foo.ts src/bar.ts` (explicit list) or `/dw-intel --build --since HEAD~5` (from git diff).

## Partial update protocol

The `intel-updater` agent receives `focus: partial --files <paths>` and:

1. **Read** the existing `.dw/intel/{files,apis,deps}.json`. Parse and keep entries for files NOT in the input list.
2. **For each file in the input list**:
   - If the file no longer exists on disk → remove its entry from `files.json` and any references in `apis.json`.
   - Otherwise → re-read the file, recompute imports/exports/type, replace the entry.
   - For `apis.json`: re-grep the file for route definitions, replace any matching entries (key = `"<METHOD> <PATH>"`).
   - For `deps.json`: re-cross-reference the file's imports. Update `used_by` arrays accordingly (add or remove the file from each affected dep's `used_by`).
3. **Skip** `stack.json` and `arch.md` — these need full context and are NOT updated by partial runs. They become stale until the next full run.
4. **Bump** `_meta.version` by 1, set `_meta.updated_at` to now.
5. **Update** `.last-refresh.json` with the new hashes for `files.json`, `apis.json`, `deps.json` (the three that were touched).

If you run a partial update on a project where `.dw/intel/` doesn't exist, abort with: `"No .dw/intel/ found. Run /dw-intel --build first for a full scan."`

## How `intel-updater` knows what's "key" in a partial

The `--files` list is authoritative for `files.json`. For `apis.json`, the agent must broaden by 1 hop:

- If `src/routes/users.ts` was in `--files`, also re-grep `src/routes/index.ts` (because it likely re-exports the user routes).
- If `src/server.ts` was in `--files`, re-scan all `src/routes/**/*.ts` (because the route registration list may have changed).

Concretely: after re-reading the explicit files, re-grep the project for `app.use(`/`router.use(`/`@Module(`/etc. and re-extract the API map from those entry points. This catches indirect changes without doing a full scan.

For `deps.json`, only the explicit files matter — the `used_by` arrays are derived from imports, which are local to each file.

## Detecting drift before updating

Use `.last-refresh.json` to detect whether files changed since the last refresh:

```bash
node -e "
const cur = require('crypto').createHash('sha256')
  .update(require('fs').readFileSync('.dw/intel/stack.json'))
  .digest('hex');
const last = require('./.dw/intel/.last-refresh.json').files['stack.json'];
console.log(cur === last ? 'unchanged' : 'changed');
"
```

If everything matches `.last-refresh.json` AND no source file's mtime is newer than `_meta.updated_at`, the index is fully fresh and the update can be a no-op.

## Conflict resolution (full update overlapping with partial)

If a full update is triggered while a partial update is in flight (rare but possible in CI), the FULL update wins:

- The partial agent's writes are overwritten when the full agent finishes.
- Both agents use atomic write (write to `.dw/intel/<file>.json.tmp` then rename) to avoid leaving the index in a torn state.

## What incremental updates do NOT cover

- New `package.json` (e.g., user added `express` to deps but no source file imports it yet) — `deps.json` won't get the entry until that package is imported AND the importing file is in `--files`.
  - Mitigation: when running `/dw-secure-audit --plan --execute`, follow up with `/dw-intel --build --full` to capture new deps.
- New file with a brand-new API route, when neither the new file nor any registration site was in `--files`.
  - Mitigation: include `src/routes/index.ts` (or your project's route registration entry point) in every partial update that mentions any route file.
- Architectural changes (the kind that would update `arch.md`) — partial updates leave `arch.md` stale.
  - Mitigation: run a full update after merging large architectural PRs.
