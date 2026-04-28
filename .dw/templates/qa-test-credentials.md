# QA Test Credentials

## Credential Profiles

| Profile | Login | Password | Role | Scope | Use When |
|---------|-------|----------|------|-------|----------|
| **Admin** | admin@test.com | {{PASSWORD}} | Administrator | Full access | Testing admin flows, user management, settings |
| **Standard User** | user@test.com | {{PASSWORD}} | User | Standard access | Testing happy paths, main user flows |
| **Restricted** | restricted@test.com | {{PASSWORD}} | Viewer | Read-only | Testing permission blocks, access denied scenarios |
| **Multi-tenant A** | tenant-a@test.com | {{PASSWORD}} | User | Tenant A | Testing tenant isolation, data segregation |
| **Multi-tenant B** | tenant-b@test.com | {{PASSWORD}} | User | Tenant B | Testing cross-tenant access blocks |

## Password Fallback Order

If the primary password fails, try in order:
1. `{{PRIMARY_PASSWORD}}`
2. `{{FALLBACK_1}}`
3. `{{FALLBACK_2}}`

If all fail, mark authentication as **BLOCKED** in the manifest.

## Login Method

- **URL:** {{LOGIN_URL}}
- **Auth provider:** {{AUTH_PROVIDER}} (e.g., Keycloak, Auth0, NextAuth)
- **Login field:** Email / Username / CPF (choose based on provider)
- **Notes:** {{NOTES}}

## Selection Guide

| Bug / Flow Type | Recommended Profile | Why |
|-----------------|--------------------|----|
| Happy path testing | Standard User | Represents typical usage |
| Permission denied tests | Restricted | Validates access control |
| Admin features | Admin | Full access needed |
| Multi-tenant isolation | Tenant A + Tenant B | Tests data boundaries |
| Auth/login flow | All profiles | Tests each access level |
