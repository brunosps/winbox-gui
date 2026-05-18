import { test, expect } from "@playwright/test";

const BASE_URL = process.env.BASE_URL ?? "{{baseUrl}}";

test("{{testTitle}}", async ({ page }) => {
  const evidence = [] as string[];

  await test.step("Open target route", async () => {
    await page.goto(`${BASE_URL}{{routePath}}`);
    await expect(page).toHaveURL(new RegExp("{{routeRegex}}"));
    evidence.push("navigation");
  });

{{testSteps}}

  await test.step("Record final context", async () => {
    expect(evidence.length).toBeGreaterThan(0);
  });
});
