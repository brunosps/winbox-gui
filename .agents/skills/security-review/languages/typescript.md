# TypeScript-Specific Security Patterns

This file complements `javascript.md` (which already covers React/Vue/Express/Next/Angular/Node XSS, injection, DOM sinks, prototype pollution). Here we focus on **TypeScript-specific** vulnerability classes: type-system abuses, runtime vs compile-time gap, TS-native ORMs, TS-native validators, and framework-specific patterns unique to TS ecosystems.

> Used by `/dw-secure-audit` as the primary reference when TS is detected in scope.

---

## The Compile-Time / Runtime Gap

TypeScript types are **erased at runtime**. Treating type annotations as validation is the most common security mistake in TS codebases.

### Always Flag

```ts
// FLAG CRITICAL: Type assertion on attacker-controlled input
const input = req.body as UserCreateDTO;
await db.user.create({ data: input }); // no runtime check — injection of extra fields

// FLAG CRITICAL: Chained "as unknown as X" to bypass checker
const user = rawJson as unknown as AdminUser; // laundering types

// FLAG HIGH: Non-null assertion on external input
const token = req.headers.authorization!.split(' ')[1]; // crash on undefined; often bypassed intent

// FLAG HIGH: any / unknown without narrowing
function handle(data: any) { db.$queryRawUnsafe(`... ${data.id}`); }
```

### Safe Patterns

```ts
// Runtime validation via zod / yup / io-ts / typebox / class-validator
import { z } from 'zod';
const UserCreate = z.object({
  email: z.string().email(),
  age: z.number().int().positive().max(150),
});
const input = UserCreate.parse(req.body); // throws on invalid

// NestJS / class-validator
import { IsEmail, IsInt, Max } from 'class-validator';
class UserCreateDTO {
  @IsEmail() email!: string;
  @IsInt() @Max(150) age!: number;
}
// combined with ValidationPipe globally applied
```

---

## Prisma (TypeScript ORM)

### Safe by Default

```ts
// SAFE: Parameterized — Prisma binds values
await db.user.findMany({ where: { email: userInput } });
await db.$queryRaw`SELECT * FROM users WHERE email = ${userInput}`;
```

### Flag These

```ts
// FLAG CRITICAL: $queryRawUnsafe with interpolation
await db.$queryRawUnsafe(`SELECT * FROM users WHERE email = '${userEmail}'`);

// FLAG HIGH: $executeRawUnsafe mutating
await db.$executeRawUnsafe(`UPDATE users SET role = '${role}' WHERE id = ${id}`);

// FLAG MEDIUM: select/where spread from req.body
await db.user.findMany({ where: req.body as any }); // attacker controls filter
```

---

## TypeORM

### Flag These

```ts
// FLAG CRITICAL: raw query() with interpolation
await manager.query(`SELECT * FROM users WHERE id = ${userId}`);

// FLAG HIGH: QueryBuilder with unsafe where()
qb.where(`user.name = '${userName}'`); // not parameterized

// SAFE equivalents
await manager.query('SELECT * FROM users WHERE id = $1', [userId]);
qb.where('user.name = :name', { name: userName });
```

---

## Drizzle ORM

### Flag These

```ts
// FLAG CRITICAL: sql template with unchecked interpolation via `sql.raw()`
import { sql } from 'drizzle-orm';
db.execute(sql.raw(`SELECT * FROM users WHERE id = ${userId}`)); // raw() bypasses binding

// SAFE: sql tagged template binds values
db.execute(sql`SELECT * FROM users WHERE id = ${userId}`); // parameterized
```

---

## NestJS

### Flag These

```ts
// FLAG HIGH: DTO without ValidationPipe or decorators
@Post() create(@Body() body: UserCreateDTO) { /* body is unvalidated */ }

// FLAG HIGH: raw ParseIntPipe missing on path param used in DB
@Get(':id') get(@Param('id') id: string) {
  return db.$queryRawUnsafe(`SELECT * FROM u WHERE id = ${id}`);
}

// FLAG CRITICAL: @Public() or missing @UseGuards on sensitive routes
@Public() @Post('admin/reset-password') reset() { /* ... */ }

// FLAG HIGH: passthrough validation with "whitelist: false"
app.useGlobalPipes(new ValidationPipe({ whitelist: false })); // mass assignment risk
```

### Safe Baseline

```ts
app.useGlobalPipes(new ValidationPipe({
  whitelist: true,           // drops unknown properties
  forbidNonWhitelisted: true,
  transform: true,
  transformOptions: { enableImplicitConversion: false },
}));
```

---

## Secrets & Environment Variables (TS-specific)

### Bundler Leaks

| Bundler | Public prefix (leaks to client bundle) |
|---------|----------------------------------------|
| Next.js | `NEXT_PUBLIC_*` |
| Vite    | `VITE_*` |
| CRA     | `REACT_APP_*` |
| Remix   | process.env used in loaders is server-only; `window.ENV` if exposed |
| Nuxt    | `NUXT_PUBLIC_*` |

```ts
// FLAG CRITICAL: Secret in public-prefixed env
NEXT_PUBLIC_STRIPE_SECRET_KEY=sk_live_...  // ships to browser bundle

// FLAG HIGH: process.env accessed in client component without server boundary
'use client';
const key = process.env.API_SECRET; // undefined OR leaked depending on bundler
```

### Unvalidated env

```ts
// FLAG HIGH: process.env.X assumed to exist
const port = parseInt(process.env.PORT); // NaN on missing

// SAFE: t3-env / zod-based env validation at boot
import { z } from 'zod';
const env = z.object({ PORT: z.coerce.number().int().positive() }).parse(process.env);
```

---

## JWT Handling in TS

### Flag These

```ts
// FLAG CRITICAL: jsonwebtoken.verify without explicit algorithms array
jwt.verify(token, secret); // allows "alg: none" if attacker controls header

// FLAG HIGH: decoded without verify
const payload = jwt.decode(token); // no signature check; decoration-only

// FLAG HIGH: tokens in localStorage from auth flow
localStorage.setItem('accessToken', token); // accessible to any XSS
```

### Safe Patterns

```ts
jwt.verify(token, secret, { algorithms: ['HS256'] });

// httpOnly + Secure + SameSite=strict cookie via Set-Cookie on the server
res.cookie('accessToken', token, {
  httpOnly: true, secure: true, sameSite: 'strict', maxAge: 15 * 60_000,
});
```

---

## Deserialization & Dynamic Evaluation

```ts
// FLAG CRITICAL
eval(userInput);
new Function('arg', userInput);
vm.runInThisContext(userInput);          // Node vm module
vm.runInNewContext(userInput, sandbox);  // sandbox escape risk

// FLAG HIGH: JSON.parse without schema, then spread into ORM
const data = JSON.parse(body);
await db.user.update({ where: { id }, data }); // mass assignment

// FLAG HIGH: YAML load unsafe (js-yaml)
import yaml from 'js-yaml';
const cfg = yaml.load(userYaml); // default can instantiate custom types in older versions
// SAFE: yaml.load with CORE_SCHEMA / FAILSAFE_SCHEMA, or use js-yaml-secure-safe alternative
```

---

## Supply Chain (TS/JS specific)

### Flag These

- `postinstall` / `prepare` scripts that fetch from network (`package.json`)
- Unpinned major versions on sensitive deps (`"express": "*"`)
- Missing `overrides` / `resolutions` when a transitive dep has a known CVE
- Typosquatting — double-check names on unfamiliar deps (historical incidents: `colors`, `node-ipc`, `event-stream`, `ua-parser-js`, `flatmap-stream`)
- `npm install` without lockfile (CI should use `npm ci` / `pnpm install --frozen-lockfile`)

### Safe Patterns

```json
// package.json
{
  "overrides": { "semver": "^7.5.2" }, // force-patch transitive CVE
  "pnpm": { "overrides": { "semver": "^7.5.2" } },
  "scripts": { "preinstall": "npx only-allow pnpm" }
}
```

---

## Node-Specific (TS)

```ts
// FLAG CRITICAL: child_process with concatenation
import { exec } from 'node:child_process';
exec(`git log ${userBranch}`); // shell injection via $(...) or ;

// SAFE: execFile with array args
import { execFile } from 'node:child_process';
execFile('git', ['log', userBranch]); // no shell

// FLAG HIGH: fs.readFile with user-controlled path
await fs.readFile(path.join('/var/data', userFile));
// SAFE: resolve + contained check
const resolved = path.resolve('/var/data', userFile);
if (!resolved.startsWith('/var/data/')) throw new Error('traversal');

// FLAG CRITICAL: dynamic require/import with user input
const mod = await import(`./plugins/${userPlugin}.js`);
```

---

## Browser-Specific (TS)

```ts
// FLAG HIGH: postMessage without origin check
window.addEventListener('message', (e) => {
  doThing(e.data); // no e.origin check
});

// FLAG HIGH: WebSocket without origin / token validation
const ws = new WebSocket(userUrl);

// FLAG MEDIUM: localStorage/sessionStorage with tokens + XSS surface
localStorage.setItem('refreshToken', token);

// FLAG HIGH: Service worker that caches opaque responses of auth'd endpoints
```

---

## Next.js 13/14 App Router Specific

```ts
// FLAG CRITICAL: server action without auth check
'use server';
export async function deleteUser(id: string) {
  await db.user.delete({ where: { id } }); // no session check
}

// FLAG HIGH: cookies() used to construct SQL
import { cookies } from 'next/headers';
const role = cookies().get('role')?.value;
await db.$queryRawUnsafe(`SELECT * FROM admin WHERE role='${role}'`);

// FLAG MEDIUM: middleware without CSP/HSTS headers
// SAFE: next.config.js headers() or middleware.ts setting strict CSP
```

---

## Research Checklist (before flagging)

For a suspected TS finding, check:

1. **Is the value actually attacker-controlled?** — trace from HTTP request / external API / file upload. Values from env or config are not attacker-controlled.
2. **Is there a validator upstream?** — search for `z.parse`, `yup.validateSync`, `class-validator`, `ajv.validate`, `ValidationPipe`.
3. **Is the ORM method actually unsafe?** — Prisma is safe by default; only `$queryRawUnsafe` / `$executeRawUnsafe` / `sql.raw()` are flags.
4. **Is the env var actually exposed to the client?** — check bundler prefix and whether the file is `'use client'`.
5. **Is the JWT verified or just decoded?** — distinguish `verify` vs `decode`.
6. **Is there CSRF protection?** — Next.js server actions, NestJS `@UseGuards`, `csurf` middleware.

Only report findings that pass the "attacker-controlled + no upstream mitigation + exploitable sink" test.

---

## Grep Patterns (TS-specific)

```bash
# Type assertion on req/body/headers
grep -rn "req\.\(body\|params\|query\|headers\).*as " --include="*.ts" --include="*.tsx"

# as unknown as chain
grep -rn "as unknown as" --include="*.ts" --include="*.tsx"

# Non-null assertion on req/external
grep -rn "req\.\(body\|params\|query\|headers\).*!" --include="*.ts"

# Prisma unsafe
grep -rn "\$queryRawUnsafe\|\$executeRawUnsafe" --include="*.ts"

# Drizzle sql.raw
grep -rn "sql\.raw(" --include="*.ts"

# TypeORM raw query
grep -rn "\.query(\`" --include="*.ts"

# jwt without algorithms
grep -rn "jwt\.verify(" --include="*.ts" | grep -v "algorithms"

# server actions without auth (Next.js 13+)
grep -rn "'use server'" --include="*.ts" --include="*.tsx"

# Bundler-exposed env prefixes with suspicious names (secret/key/token/password)
grep -rEn "(NEXT_PUBLIC_|VITE_|REACT_APP_|NUXT_PUBLIC_)\w*(SECRET|KEY|TOKEN|PASS|PRIV)" --include="*.env*"

# zod schema presence (for context, not flag)
grep -rn "z\.object\|\.parse(" --include="*.ts"
```

---

## Cross-Reference

For XSS/injection/CSRF/SSRF/crypto patterns that are NOT TS-specific, see:
- `../references/xss.md`
- `../references/injection.md`
- `../references/csrf.md`
- `../references/ssrf.md`
- `../references/cryptography.md`
- `../references/authentication.md`
- `../references/authorization.md`
- `../references/supply-chain.md`
- `./javascript.md` — React/Vue/Express/Angular/Node generic JS patterns
