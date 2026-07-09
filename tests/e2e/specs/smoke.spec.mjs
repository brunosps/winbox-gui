import { expect } from "expect-webdriverio";

// Smoke test: verifies the Tauri app boots, the dashboard renders, and the
// "Novo perfil" button is reachable. Does NOT exercise the launch flow
// (that needs a Docker mock — see tests/e2e/fixtures/mock-docker.sh and
// the TODO in docs/E2E-SETUP.md).

describe("winbox dashboard smoke", () => {
  it("loads the main window with the brand label", async () => {
    // Wait for the WebView to be reachable.
    await browser.pause(2000);

    // The current header renders the product name as the brand heading.
    const brand = await $(".brand h1");
    await brand.waitForExist({ timeout: 10_000 });
    await expect(brand).toBeDisplayed();
    await expect(brand).toHaveText(expect.stringContaining("winbox"));
  });

  it("shows the New Profile button", async () => {
    const btn = await $("#btn-install");
    await btn.waitForExist({ timeout: 5_000 });
    await expect(btn).toBeDisplayed();
  });

  it("renders an empty dashboard or fixture profiles", async () => {
    // With the isolated XDG_CONFIG_HOME pointing at the fixture, the only
    // profiles visible are the ones written by the fixture setup. If no
    // fixture profiles are seeded, the dashboard shows the empty-state
    // illustration.
    const rows = await $$(".profile-row");
    const empty = await $(".table-empty");
    const hasContent = rows.length > 0 || (await empty.isExisting());
    expect(hasContent).toBe(true);
  });
});
