# Padrões de Teste Playwright

Referência para `/dw-qa` e `/dw-functional-doc`. Padrões E2E comuns.

## 1. Navegação Autenticada

```typescript
import { test, expect } from "@playwright/test";

test("navegar rota autenticada", async ({ page }) => {
  // Login
  await page.goto("/login");
  await page.getByLabel("Email").fill("user@test.com");
  await page.getByLabel("Senha").fill("password123");
  await page.getByRole("button", { name: "Entrar" }).click();
  
  // Aguardar redirect
  await page.waitForURL("/dashboard");
  await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();
});
```

## 2. Submissão de Formulário com Validação

```typescript
test("submeter formulário com erros de validação", async ({ page }) => {
  await page.goto("/users/new");

  // Submeter vazio → erros de validação
  await page.getByRole("button", { name: "Salvar" }).click();
  await expect(page.getByText("Nome é obrigatório")).toBeVisible();
  await expect(page.getByText("Email é obrigatório")).toBeVisible();

  // Preencher e submeter → sucesso
  await page.getByLabel("Nome").fill("Maria Silva");
  await page.getByLabel("Email").fill("maria@example.com");
  await page.getByRole("button", { name: "Salvar" }).click();
  
  await expect(page.getByText("Usuário criado com sucesso")).toBeVisible();
});
```

## 3. Tabela com Filtro e Paginação

```typescript
test("filtrar e paginar tabela", async ({ page }) => {
  await page.goto("/users");

  // Verificar carregamento inicial
  const rows = page.locator("table tbody tr");
  await expect(rows).toHaveCount(10);

  // Filtrar
  await page.getByPlaceholder("Buscar...").fill("admin");
  await expect(rows).toHaveCount(2);

  // Limpar e paginar
  await page.getByPlaceholder("Buscar...").clear();
  await page.getByRole("button", { name: "Próxima" }).click();
  await expect(page.getByText("Página 2")).toBeVisible();
});
```

## 4. Interação com Modal / Dialog

```typescript
test("abrir modal, preencher e confirmar", async ({ page }) => {
  await page.goto("/projects");

  // Abrir modal
  await page.getByRole("button", { name: "Novo Projeto" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  // Preencher formulário do modal
  await dialog.getByLabel("Nome do Projeto").fill("Meu Projeto");
  await dialog.getByRole("button", { name: "Criar" }).click();

  // Modal fecha, item aparece na lista
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText("Meu Projeto")).toBeVisible();
});
```

## 5. Permissão / Acesso Negado

```typescript
test("usuário restrito vê acesso negado", async ({ page }) => {
  // Login como usuário restrito
  await page.goto("/login");
  await page.getByLabel("Email").fill("restricted@test.com");
  await page.getByLabel("Senha").fill("password123");
  await page.getByRole("button", { name: "Entrar" }).click();

  // Tentar acessar rota admin
  await page.goto("/admin/settings");
  
  // Verificar acesso negado
  await expect(page.getByText(/acesso negado|proibido|não autorizado/i)).toBeVisible();
});
```

## 6. Tratamento de Erro de API

```typescript
test("trata erro de API graciosamente", async ({ page }) => {
  // Interceptar API para simular erro
  await page.route("**/api/users", (route) =>
    route.fulfill({ status: 500, body: "Internal Server Error" })
  );

  await page.goto("/users");

  // Verificar estado de erro
  await expect(page.getByText(/erro|algo deu errado/i)).toBeVisible();
  
  // Verificar que botão de retry funciona
  await page.unroute("**/api/users");
  await page.getByRole("button", { name: /tentar novamente/i }).click();
  await expect(page.locator("table tbody tr")).toHaveCount(10);
});
```

## Padrão de Captura de Evidência

```typescript
await test.step("Capturar evidência", async () => {
  await page.screenshot({ 
    path: `evidence/screenshots/${testInfo.title}-${Date.now()}.png`, 
    fullPage: true 
  });
});
```
