# Intel format — schemas of `.dw/intel/` files

Every command that reads `.dw/intel/` depends on these schemas. Treat this file as the contract.

## Layout

```
.dw/intel/
├── stack.json           # languages, frameworks, build/test tooling
├── files.json           # per-file imports/exports/type
├── apis.json            # routes, endpoints, CLI commands
├── deps.json            # third-party dependencies + usage
├── arch.md              # architectural overview (markdown)
└── .last-refresh.json   # change detection (timestamps + sha256 hashes)
```

## Common `_meta` block (all JSON files)

```json
{
  "_meta": {
    "updated_at": "2026-05-06T12:34:56.789Z",
    "version": 3
  }
}
```

- `updated_at`: ISO-8601 UTC timestamp, set on every write
- `version`: integer, starts at 1, increments on every update (full or partial)

## `stack.json`

```json
{
  "_meta": { "updated_at": "...", "version": 1 },
  "languages": ["TypeScript", "JavaScript"],
  "frameworks": ["Express", "React"],
  "tools": ["ESLint", "Jest", "Docker"],
  "build_system": "npm scripts",
  "test_framework": "Jest",
  "package_manager": "npm",
  "content_formats": ["Markdown", "YAML", "EJS"]
}
```

Field rules:
- `languages`: ordered by line count (most-prevalent first)
- `frameworks`: full names (no abbreviations) — `"Next.js"` not `"next"`
- `tools`: dev tooling that affects code shape (linter, formatter, bundler, container runtime)
- `build_system`: detected from manifest (`npm scripts`, `Vite`, `webpack`, `esbuild`, `dotnet`, `cargo`, etc.)
- `test_framework`: detected from manifest (`Jest`, `Vitest`, `Pytest`, `xUnit`, `cargo test`, etc.); `"none"` if no test setup detected
- `package_manager`: from lockfile (`npm`, `pnpm`, `yarn`, `poetry`, `uv`, `dotnet`, `cargo`)
- `content_formats`: non-code formats with structural significance (markdown for docs/skills, YAML for configs, JSON Schema, OpenAPI, Protobuf, etc.)

## `files.json`

```json
{
  "_meta": { "updated_at": "...", "version": 1 },
  "entries": {
    "src/server.ts": {
      "exports": ["createServer", "default"],
      "imports": ["./config", "express", "./routes/users"],
      "type": "entry-point"
    },
    "src/routes/users.ts": {
      "exports": ["userRouter"],
      "imports": ["express", "../db", "../schemas/user"],
      "type": "module"
    }
  }
}
```

Field rules:
- Key: file path relative to project root, forward slashes
- `exports`: array of ACTUAL exported symbol names. NOT descriptions. `["createServer", "default"]` is correct; `["server creation"]` is wrong.
- `imports`: array of import specifiers (relative paths or package names) — preserve as the source code writes them
- `type`: one of `entry-point`, `module`, `config`, `test`, `script`, `type-def`, `style`, `template`, `data`

Coverage: target the most important 50-100 files. Skip generated code, test files (unless they reveal architecture), `node_modules/dist/build`.

## `apis.json`

```json
{
  "_meta": { "updated_at": "...", "version": 1 },
  "entries": {
    "GET /api/users": {
      "method": "GET",
      "path": "/api/users",
      "params": ["page", "limit"],
      "file": "src/routes/users.ts",
      "description": "List users with pagination"
    },
    "CLI: dw-init": {
      "method": "CLI",
      "path": "dev-workflow init",
      "params": ["--lang", "--force"],
      "file": "bin/dev-workflow.js",
      "description": "Scaffold .dw/ structure into a project"
    }
  }
}
```

Field rules:
- Key format: `"<METHOD> <PATH>"` for HTTP, `"CLI: <command>"` for CLI, `"GraphQL: <operation>"`, `"Handler: <event>"`, etc.
- `method`: HTTP verb, or `"CLI"`, `"GraphQL"`, `"gRPC"`, `"Event"`, `"Subscription"`
- `path`: the route/command/operation name
- `params`: query/body/CLI flag names — array of strings
- `file`: source path
- `description`: one-line summary derived from the handler's first comment or function name

If no API surfaces detected, write `{ "_meta": {...}, "entries": {} }` — never omit the file.

## `deps.json`

```json
{
  "_meta": { "updated_at": "...", "version": 1 },
  "entries": {
    "express": {
      "version": "^4.18.0",
      "type": "production",
      "used_by": ["src/server.ts", "src/routes/users.ts"],
      "invocation": "require"
    },
    "vitest": {
      "version": "^2.0.0",
      "type": "development",
      "used_by": ["package.json#test"],
      "invocation": "npm test"
    }
  }
}
```

Field rules:
- Key: package name (npm), distribution name (PyPI), package id (NuGet), crate name (crates.io)
- `version`: semver range from manifest
- `type`: `production` | `development` | `peer` | `optional`
- `used_by`: array of file paths that import the dep (verified by Grep) OR npm-script keys for tooling deps
- `invocation`:
  - `require` if directly imported in code
  - npm script command (`npm run lint`, `npm test`) if exercised via script
  - `implicit` if a runtime/framework dep that's not directly imported (e.g., `react-dom` in a Next.js project)

## `arch.md`

```markdown
---
updated_at: "2026-05-06T12:34:56.789Z"
---

## Architecture Overview

The project follows a layered architecture: HTTP routes → application services → repository layer → ORM → database.

## Key Components

| Component | Path | Responsibility |
|-----------|------|----------------|
| Server bootstrap | `src/server.ts` | Express app initialization + middleware wiring |
| Route handlers | `src/routes/` | HTTP request validation and response shaping |
| Application services | `src/services/` | Business logic, orchestration |
| Repositories | `src/repositories/` | Data access via Prisma |
| Schemas | `src/schemas/` | Zod validation schemas, shared with frontend |

## Data Flow

`HTTP request` → `route handler (validates with Zod)` → `service` → `repository (Prisma)` → `Postgres` → response

## Conventions

- File naming: kebab-case (`user-routes.ts`, `order-service.ts`)
- Function naming: camelCase, verb-first (`createOrder`, `findUserById`)
- Tests: co-located, `.test.ts` suffix
- Imports: absolute via `@/` alias for cross-module, relative for sibling files
```

## `.last-refresh.json`

```json
{
  "updated_at": "2026-05-06T12:34:56.789Z",
  "files": {
    "stack.json": "a3c8b1...",
    "files.json": "f9e2d7...",
    "apis.json": "1b4f8a...",
    "deps.json": "5c2e6b...",
    "arch.md": "9d7c3e..."
  }
}
```

Hashes are SHA-256 of file contents at the moment of refresh. Used by `/dw-intel --build` to detect drift on the next run and decide whether a partial update is enough.

## Validation rules

When `intel-updater` finishes, all 5 files MUST satisfy:

1. Valid JSON (parseable)
2. `_meta.updated_at` is ISO-8601
3. `_meta.version` is a positive integer
4. All file paths in `files.json`, `apis.json`, `deps.json` exist on disk (or were excluded for known reasons)
5. `exports` arrays contain only actual symbols (no descriptions, no spaces in entries)
6. No secrets/credentials/tokens in any file
