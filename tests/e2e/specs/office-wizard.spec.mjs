import { expect } from "expect-webdriverio";
import { spawnSync } from "node:child_process";
import { accessSync, constants, existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const e2eRoot = resolve(__dirname, "..");
const fixturesDir = resolve(e2eRoot, "fixtures");
const shimsDir = resolve(fixturesDir, "shims");
const shimLog = resolve(fixturesDir, "xdg/logs/shims.log");

describe("office wizard e2e harness", () => {
  it("e2e_harness_uses_path_shims_not_host_binaries", async () => {
    const required = [
      "docker",
      "docker-compose",
      "flatpak",
      "git",
      "curl",
      "xfreerdp",
      "xfreerdp3",
      "winapps-setup",
      "notify-send",
      "ip",
    ];
    for (const name of required) {
      accessSync(resolve(shimsDir, name), constants.X_OK);
    }
    accessSync(resolve(fixturesDir, "mock-docker.sh"), constants.X_OK);

    const docker = spawnSync(resolve(shimsDir, "docker"), ["info"], { encoding: "utf8" });
    expect(docker.status).toBe(0);
    expect(docker.stdout).toContain("Docker Engine");

    const configText = readFileSync(resolve(e2eRoot, "wdio.conf.mjs"), "utf8");
    expect(configText).toContain("fixtures/shims");
    expect(configText).toContain("mock-docker.sh");

    await waitForDashboard();
    await browser.waitUntil(
      () => existsSync(shimLog) && readFileSync(shimLog, "utf8").includes("docker ps"),
      {
        timeout: 5_000,
        timeoutMsg: "expected app Docker status checks to go through the E2E docker shim",
      },
    );
  });

  it("e2e_office_wizard_shows_duration_expectation", async () => {
    await openOfficeWizardWithMockHarness();
    await driveToPreflight();

    await expect($(".office-wizard-preflight-check[data-status='warning']")).toBeDisplayed();
    await expect($("[data-office-wizard-action='start']")).toBeDisabled();

    await $("[data-office-wizard-field='warningOverride']").click();
    await expect($("[data-office-wizard-action='start']")).toBeEnabled();
    await $("[data-office-wizard-action='start']").click();

    const intro = await $("[data-testid='office-provisioning-intro']");
    await intro.waitForDisplayed({ timeout: 5_000 });
    await expect(intro).toHaveText(expect.stringContaining("45 minutes"));
    await expect(intro).toHaveText(expect.stringContaining("Large downloads"));
  });

  it("e2e_office_wizard_mock_provisioning_reaches_ready", async () => {
    await openOfficeWizardWithMockHarness();
    await driveToPreflight();
    await $("[data-office-wizard-field='warningOverride']").click();
    await $("[data-office-wizard-action='start']").click();

    const failure = await $(".office-wizard-callout[data-tone='danger']");
    await failure.waitForDisplayed({ timeout: 5_000 });
    await expect(failure).toHaveText(expect.stringContaining("office_odt_failed"));

    await browser.execute(() => window.__winboxE2eOfficeWizard.retry());

    const title = await $("#office-wizard-current-title");
    await title.waitUntil(
      async function waitForReadyTitle() {
        return (await this.getText()).includes("Office is installed");
      },
      { timeout: 5_000, timeoutMsg: "Office wizard did not reach ready state" },
    );
    await expect($("[data-office-wizard-action='launch-app'][data-office-app='excel']")).toBeDisplayed();

    const a11y = await browser.execute(() => {
      const panel = document.querySelector(".office-wizard-panel");
      return {
        live: panel?.getAttribute("aria-live"),
        hasFocusable: Boolean(document.querySelector(".office-wizard button, .office-wizard input, .office-wizard select")),
        hasNegativeTabindex: Boolean(document.querySelector('.office-wizard [tabindex="-1"]')),
        horizontalOverflow: document.documentElement.scrollWidth > window.innerWidth + 1,
      };
    });
    expect(a11y.live).toBe("polite");
    expect(a11y.hasFocusable).toBe(true);
    expect(a11y.hasNegativeTabindex).toBe(false);
    expect(a11y.horizontalOverflow).toBe(false);
  });
});

async function waitForDashboard() {
  const brand = await $(".brand h1");
  await brand.waitForDisplayed({ timeout: 10_000 });
  await expect(brand).toHaveText(expect.stringContaining("winbox"));
  await $("[data-office-wizard-open]").waitForDisplayed({ timeout: 10_000 });
}

async function openOfficeWizardWithMockHarness() {
  await browser.setWindowSize(1024, 768);
  await waitForDashboard();
  await $("[data-office-wizard-open]").click();
  await $(".office-wizard").waitForDisplayed({ timeout: 5_000 });
  await browser.pause(100);
  const installed = await browser.executeAsync(async (done) => {
    try {
      const wizard = await import("./office-wizard.js");
      const dom = await import("./dom-utils.js");
      const i18n = await import("./i18n.js");
      await import("./locales/en-US.js");
      i18n.setLocale("en-US");

      const main = document.querySelector("#main");
      let state = wizard.initialOfficeWizardState();
      let unbind = null;
      const deps = {
        t: i18n.t,
        escapeHtml: dom.escapeHtml,
        escapeAttr: dom.escapeAttr,
      };
      const warningPreflight = {
        checks: [
          {
            id: "preflight_resources_ram",
            status: "warning",
            requirement: "8 GB RAM recommended",
            impact: "Office can run slowly with constrained RAM.",
            action_hint: "Confirm the warning override to continue.",
          },
          {
            id: "preflight_resources_disk",
            status: "warning",
            requirement: "128 GB disk recommended",
            impact: "Large Office downloads and Windows updates need disk margin.",
            action_hint: "Confirm the warning override to continue.",
          },
        ],
        warnings: 2,
        blockers: 0,
        adoptionCandidates: [],
      };

      function render() {
        if (unbind) unbind();
        main.innerHTML = wizard.renderOfficeWizard(state, deps);
        unbind = wizard.bindOfficeWizard(main, {
          getState: () => state,
          dispatch(action) {
            state = wizard.officeWizardReducer(state, action);
            if (action.type === "next" && state.step === "preflight") {
              state = wizard.officeWizardReducer(state, {
                type: "set_preflight_result",
                result: warningPreflight,
              });
            }
            render();
          },
          onStart: startMockProvisioning,
          onClose() {},
          onLaunchApp() {},
        });
        i18n.applyAll(main);
      }

      function progress(step, status, message, code = "") {
        state = wizard.applyOfficeProgressEvent(state, {
          type: "operation-progress",
          profile: state.profileName,
          op: "office_provision",
          step,
          status,
          message,
          code,
          timestamp: new Date().toISOString(),
        });
        render();
      }

      function startMockProvisioning() {
        state = wizard.officeWizardReducer(state, { type: "set_status", status: "loading" });
        state = wizard.officeWizardReducer(state, { type: "set_step", step: "provisioning" });
        render();
        progress("office_odt_stage", "running", "Staging Office Deployment Tool.");
        progress(
          "office_odt_stage",
          "error",
          "Mocked office_odt_failed while staging Office Deployment Tool.",
          "office_odt_failed",
        );
      }

      function retry() {
        progress("office_odt_stage", "running", "Retrying Office Deployment Tool staging.");
        progress("office_odt_stage", "success", "Office Deployment Tool staged.");
        progress("office_odt_install", "running", "Installing Office payload.");
        progress("office_odt_install", "success", "Office installed.");
        progress("office_winapps_setup", "success", "WinApps configured.");
        progress("office_desktop_register", "success", "Desktop entries registered.");
        progress("office_file_association", "success", "Office file associations registered.");
        progress("office_final_verify", "success", "Final verification complete.");
      }

      window.__winboxE2eOfficeWizard = {
        retry,
        getState: () => JSON.parse(JSON.stringify(state)),
      };
      render();
      done({ ok: true });
    } catch (err) {
      done({ ok: false, message: String(err?.stack || err) });
    }
  });
  expect(installed.ok).toBe(true);
}

async function driveToPreflight() {
  await $("[data-office-wizard-action='next']").click();
  await $("#office-profile-name").waitForDisplayed({ timeout: 5_000 });
  await $("[data-office-wizard-action='next']").click();
  await $("[data-office-wizard-field='byolAccepted']").waitForDisplayed({ timeout: 5_000 });
  await $("[data-office-wizard-field='byolAccepted']").click();
  await $("[data-office-wizard-action='next']").click();
  await $(".office-wizard-preflight-list").waitForDisplayed({ timeout: 5_000 });
}
