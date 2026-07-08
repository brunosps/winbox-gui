# Recipe: `supertest` (Node.js / TypeScript)

Use for Fastify, Express, NestJS projects that already run `vitest` or `jest`. supertest binds directly to the app instance and runs in-process — no port allocation, no flake.

## File shape

`{{PRD_PATH}}/QA/scripts/api/RF-XX-[slug].test.ts`

```ts
// RF-XX [slug] — API QA suite
import request from 'supertest';
import { describe, expect, it, beforeAll } from 'vitest';
import { buildApp } from '../../../src/app'; // or: import app from '../../../src/server';

const BASE = process.env.API_BASE_URL ?? 'http://localhost:3000';
const TOKEN_ADMIN = process.env.QA_TOKEN_ADMIN ?? '';
const TOKEN_USER = process.env.QA_TOKEN_USER ?? '';
const TOKEN_OTHER_ORG = process.env.QA_TOKEN_OTHER_ORG ?? '';

let app: Awaited<ReturnType<typeof buildApp>>;
let createdUserId: string;

beforeAll(async () => { app = await buildApp(); });

const auth = (t: string) => ({ Authorization: `Bearer ${t}` });

describe('RF-XX create user', () => {

  it('happy path returns 201 and id', async () => {
    const r = await request(app.server).post('/users')
      .set(auth(TOKEN_ADMIN))
      .send({ email: `qa-${Date.now()}@example.com`, name: 'QA' });
    expect(r.status).toBe(201);
    expect(r.body.id).toBeDefined();
    createdUserId = r.body.id;
  });

  it.each([
    [{ name: 'No email' }, 'email'],
    [{ email: 'no-name@x.com' }, 'name'],
    [{ email: 'not-an-email', name: 'X' }, 'email'],
  ])('validation: %j → mentions %s', async (payload, missing) => {
    const r = await request(app.server).post('/users')
      .set(auth(TOKEN_ADMIN))
      .send(payload);
    expect(r.status).toBe(422);
    expect(r.body.error.message.toLowerCase()).toContain(missing);
  });

  it('no token returns 401', async () => {
    const r = await request(app.server).post('/users')
      .send({ email: 'x@y.com', name: 'x' });
    expect(r.status).toBe(401);
  });

  it('expired token returns 401', async () => {
    const r = await request(app.server).post('/users')
      .set({ Authorization: 'Bearer expired.token.here' })
      .send({ email: 'x@y.com', name: 'x' });
    expect(r.status).toBe(401);
  });

  it('cross-tenant access denied', async () => {
    if (!TOKEN_OTHER_ORG) return;
    const r = await request(app.server).get(`/users/${createdUserId}`)
      .set(auth(TOKEN_OTHER_ORG));
    expect([403, 404]).toContain(r.status);
  });

  it('contract: required fields present, leaked fields absent', async () => {
    const r = await request(app.server).get(`/users/${createdUserId}`)
      .set(auth(TOKEN_ADMIN));
    expect(r.status).toBe(200);
    for (const f of ['id', 'email', 'name', 'created_at']) {
      expect(r.body[f]).toBeDefined();
    }
    for (const f of ['password_hash', 'internal_id', '_raw']) {
      expect(r.body[f]).toBeUndefined();
    }
  });
});
```

## Configuration

`vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    include: ['QA/scripts/api/**/*.test.ts'],
    testTimeout: 10_000,
    hookTimeout: 30_000,
  },
});
```

## Running

```bash
# all
pnpm vitest run QA/scripts/api

# one RF
pnpm vitest run QA/scripts/api/RF-01-create-user.test.ts

# log to QA/logs/api/
pnpm vitest run QA/scripts/api 2>&1 | tee "QA/logs/api/run-$(date +%F).log"
```

## Logging request/response

Wrap the supertest agent in a small helper that emits to JSONL:

```ts
import fs from 'node:fs';

const LOG = 'QA/logs/api/RF-XX-create-user.log';
fs.mkdirSync('QA/logs/api', { recursive: true });

function logRequest(method: string, url: string, status: number, ms: number, reqBody: unknown, resBody: unknown) {
  fs.appendFileSync(LOG, JSON.stringify({
    ts: Date.now(), method, url, status, ms, request_body: reqBody, response_body: resBody,
  }) + '\n');
}
```

## NestJS variant

Use `@nestjs/testing`'s `Test.createTestingModule(...)` + `app.getHttpServer()` instead of `buildApp`:

```ts
import { Test } from '@nestjs/testing';
import { AppModule } from '../../../src/app.module';

let app: INestApplication;
beforeAll(async () => {
  const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
  app = moduleRef.createNestApplication();
  await app.init();
});

// then: request(app.getHttpServer()) ...
```

## Pros / cons

- **Pro**: in-process, no port allocation, fastest possible.
- **Pro**: integrates with `vitest`/`jest` watch + coverage.
- **Pro**: `it.each` covers the 4xx matrix in one block.
- **Con**: only works against a `supertest`-compatible HTTP framework.
- **Con**: requires the `buildApp` factory pattern; one-off scripts/handlers may need refactor.
