# Credenciais de Teste QA

## Perfis de Credenciais

| Perfil | Login | Senha | Role | Escopo | Usar Quando |
|--------|-------|-------|------|--------|-------------|
| **Admin** | admin@test.com | {{PASSWORD}} | Administrador | Acesso total | Testar fluxos admin, gestão de usuários, configurações |
| **Usuário Padrão** | user@test.com | {{PASSWORD}} | Usuário | Acesso padrão | Testar happy paths, fluxos principais |
| **Restrito** | restricted@test.com | {{PASSWORD}} | Visualizador | Somente leitura | Testar bloqueios de permissão, acesso negado |
| **Multi-tenant A** | tenant-a@test.com | {{PASSWORD}} | Usuário | Tenant A | Testar isolamento de tenant, segregação de dados |
| **Multi-tenant B** | tenant-b@test.com | {{PASSWORD}} | Usuário | Tenant B | Testar bloqueio de acesso cross-tenant |

## Ordem de Fallback de Senha

Se a senha primária falhar, tente nesta ordem:
1. `{{PRIMARY_PASSWORD}}`
2. `{{FALLBACK_1}}`
3. `{{FALLBACK_2}}`

Se todas falharem, marcar autenticação como **BLOQUEADA** no manifesto.

## Método de Login

- **URL:** {{LOGIN_URL}}
- **Provedor de autenticação:** {{AUTH_PROVIDER}} (ex: Keycloak, Auth0, NextAuth)
- **Campo de login:** Email / Usuário / CPF (escolher baseado no provedor)
- **Observações:** {{NOTES}}

## Guia de Seleção

| Tipo de Bug / Fluxo | Perfil Recomendado | Motivo |
|---------------------|-------------------|--------|
| Teste de happy path | Usuário Padrão | Representa uso típico |
| Teste de permissão negada | Restrito | Valida controle de acesso |
| Funcionalidades admin | Admin | Acesso total necessário |
| Isolamento multi-tenant | Tenant A + Tenant B | Testa fronteiras de dados |
| Fluxo de auth/login | Todos os perfis | Testa cada nível de acesso |
