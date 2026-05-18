# Playwright Test Patterns

Reference for `/dw-run-qa` and `/dw-functional-doc`. Common E2E patterns.

## 1. Authenticated Navigation

```typescript
import { test, expect } from "@playwright/test";

test("navigate authenticated route", async ({ page }) => {
  // Login
  await page.goto("/login");
  await page.getByLabel("Email").fill("user@test.com");
  await page.getByLabel("Password").fill("password123");
  await page.getByRole("button", { name: "Sign in" }).click();
  
  // Wait for redirect
  await page.waitForURL("/dashboard");
  await expect(page.getByRole("heading", { name: /dashboard/i })).toBeVisible();
});
```

## 2. Form Submission with Validation

```typescript
test("submit form with validation errors", async ({ page }) => {
  await page.goto("/users/new");

  // Submit empty → validation errors
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Name is required")).toBeVisible();
  await expect(page.getByText("Email is required")).toBeVisible();

  // Fill and submit → success
  await page.getByLabel("Name").fill("Jane Doe");
  await page.getByLabel("Email").fill("jane@example.com");
  await page.getByRole("button", { name: "Save" }).click();
  
  await expect(page.getByText("User created successfully")).toBeVisible();
});
```

## 3. Table with Filtering and Pagination

```typescript
test("filter and paginate table", async ({ page }) => {
  await page.goto("/users");

  // Verify initial load
  const rows = page.locator("table tbody tr");
  await expect(rows).toHaveCount(10); // default page size

  // Filter
  await page.getByPlaceholder("Search...").fill("admin");
  await expect(rows).toHaveCount(2); // filtered results

  // Clear and paginate
  await page.getByPlaceholder("Search...").clear();
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByText("Page 2")).toBeVisible();
});
```

## 4. Modal / Dialog Interaction

```typescript
test("open modal, fill form, confirm", async ({ page }) => {
  await page.goto("/projects");

  // Open modal
  await page.getByRole("button", { name: "New Project" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  // Fill modal form
  await dialog.getByLabel("Project Name").fill("My Project");
  await dialog.getByRole("button", { name: "Create" }).click();

  // Modal closes, item appears in list
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText("My Project")).toBeVisible();
});
```

## 5. Permission / Access Denied

```typescript
test("restricted user sees access denied", async ({ page }) => {
  // Login as restricted user
  await page.goto("/login");
  await page.getByLabel("Email").fill("restricted@test.com");
  await page.getByLabel("Password").fill("password123");
  await page.getByRole("button", { name: "Sign in" }).click();

  // Try to access admin route
  await page.goto("/admin/settings");
  
  // Verify access denied
  await expect(page.getByText(/access denied|forbidden|not authorized/i)).toBeVisible();
  // OR redirect to dashboard
  await expect(page).toHaveURL(/dashboard/);
});
```

## 6. API Error Handling

```typescript
test("handles API error gracefully", async ({ page }) => {
  // Intercept API to simulate error
  await page.route("**/api/users", (route) =>
    route.fulfill({ status: 500, body: "Internal Server Error" })
  );

  await page.goto("/users");

  // Verify error state
  await expect(page.getByText(/error|something went wrong/i)).toBeVisible();
  
  // Verify retry button works
  await page.unroute("**/api/users");
  await page.getByRole("button", { name: /retry|try again/i }).click();
  await expect(page.locator("table tbody tr")).toHaveCount(10);
});
```

## Evidence Capture Pattern

```typescript
// Use in any test to capture evidence
await test.step("Capture evidence", async () => {
  await page.screenshot({ 
    path: `evidence/screenshots/${testInfo.title}-${Date.now()}.png`, 
    fullPage: true 
  });
});
```
