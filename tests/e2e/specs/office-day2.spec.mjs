import { expect } from "expect-webdriverio";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const e2eRoot = resolve(__dirname, "..");
const fixturesDir = resolve(e2eRoot, "fixtures");
const xdgRoot = resolve(fixturesDir, "xdg");
const xdgConfigHome = resolve(xdgRoot, "config");
const xdgDataHome = resolve(xdgRoot, "data");
const shimLogDir = resolve(xdgRoot, "logs");
const upgradeFallbackLog = resolve(shimLogDir, "upgrade-fallback.log");

describe("office day-2 e2e flows", () => {
  it("e2e_office_launch_shows_cold_start_progress", async () => {
    await installLaunchProgressHarness();

    await browser.execute(() => {
      window.__winboxE2eOfficeLaunch.emit(
        "office_cold_start",
        "running",
        "Starting stopped VM before launching Excel.",
      );
    });
    await expect($(".office-progress-window-status")).toHaveText(expect.stringContaining("Starting stopped VM"));

    await browser.execute(() => {
      window.__winboxE2eOfficeLaunch.emit(
        "office_cold_start",
        "success",
        "VM is ready for RemoteApp.",
      );
      window.__winboxE2eOfficeLaunch.emit(
        "office_launch_remoteapp",
        "running",
        "Opening Excel through the WinApps RemoteApp launcher.",
      );
    });
    await expect($(".office-progress-window-status")).toHaveText(expect.stringContaining("Opening Excel"));

    await browser.execute(() => {
      window.__winboxE2eOfficeLaunch.emit(
        "office_launch_remoteapp",
        "success",
        "Delegated Excel to WinApps.",
      );
    });

    const state = await browser.execute(() => window.__winboxE2eOfficeLaunch.state());
    expect(state.wizardState.phaseStatuses.first_launch).toBe("done");
    await expect($(".office-wizard-phases li[data-status='done']")).toBeDisplayed();
  });

  it("e2e_office_lifecycle_warns_when_apps_maybe_open", async () => {
    await installLifecycleHarness();

    for (const action of ["pause", "stop", "restart"]) {
      await $(`[data-office-lifecycle-action='${action}']`).click();
      await $("#modal-confirm.open").waitForDisplayed({ timeout: 5_000 });
      await expect($("#confirm-message")).toHaveText(expect.stringContaining("office_apps_maybe_open"));
      await $("#confirm-yes").click();
    }

    await browser.execute(() => window.__winboxE2eLifecycle.setActiveSessions(null));
    await $("[data-office-lifecycle-action='stop']").click();
    await $("#modal-confirm.open").waitForDisplayed({ timeout: 5_000 });
    await expect($("#confirm-message")).toHaveText(expect.stringContaining("session status is indeterminate"));
    await $("#confirm-yes").click();

    const actions = await browser.execute(() => window.__winboxE2eLifecycle.actions());
    expect(actions).toEqual([
      "pause:confirmed:office_apps_maybe_open",
      "stop:confirmed:office_apps_maybe_open",
      "restart:confirmed:office_apps_maybe_open",
      "stop:confirmed:office_apps_maybe_open",
    ]);
  });

  it("e2e_office_upgrade_preserves_state_menu_and_associations", () => {
    const before = seedOfficeUpgradeFixture();

    simulateUpgradeFallback("0.1.0", "0.1.1");

    const after = officeUpgradeSnapshot();
    expect(after.state.profileKind).toBe("office");
    expect(after.state.status).toBe("ready");
    expect(after.state.phases.final_verify.status).toBe("done");
    expect(after.desktop).toEqual(before.desktop);
    expect(after.mimeapps).toEqual(before.mimeapps);
    expect(readFileSync(upgradeFallbackLog, "utf8")).toContain(
      "simulated upgrade fallback: swapped binary marker from 0.1.0 to 0.1.1",
    );
  });
});

async function waitForDashboard() {
  await browser.setWindowSize(1024, 768);
  const brand = await $(".brand h1");
  await brand.waitForDisplayed({ timeout: 10_000 });
  await expect(brand).toHaveText(expect.stringContaining("winbox"));
}

async function installLaunchProgressHarness() {
  await waitForDashboard();
  const installed = await browser.executeAsync(async (done) => {
    try {
      const progressWindow = await import("./office-progress-window.js");
      const dom = await import("./dom-utils.js");
      const i18n = await import("./i18n.js");
      await import("./locales/en-US.js");
      i18n.setLocale("en-US");

      const main = document.querySelector("#main");
      let state = progressWindow.initialOfficeProgressWindowState({
        profileName: "office",
        appId: "excel",
      });
      const deps = {
        t: i18n.t,
        escapeHtml: dom.escapeHtml,
        escapeAttr: dom.escapeAttr,
      };

      function render() {
        main.innerHTML = progressWindow.renderOfficeProgressWindow(state, deps);
        i18n.applyAll(main);
      }

      function emit(step, status, message) {
        state = progressWindow.officeProgressWindowReducer(state, {
          type: "operation_progress",
          event: {
            type: "operation-progress",
            profile: "office",
            operation: "office_launch",
            step,
            status,
            message,
            timestamp: new Date().toISOString(),
          },
        });
        render();
      }

      window.__winboxE2eOfficeLaunch = {
        emit,
        state: () => JSON.parse(JSON.stringify(state)),
      };
      render();
      done({ ok: true });
    } catch (err) {
      done({ ok: false, message: String(err?.stack || err) });
    }
  });
  expect(installed.ok).toBe(true);
}

async function installLifecycleHarness() {
  await waitForDashboard();
  const installed = await browser.execute(() => {
    const main = document.querySelector("#main");
    const modal = document.querySelector("#modal-confirm");
    const originalConfirm = document.querySelector("#confirm-yes");
    const confirmButton = originalConfirm.cloneNode(true);
    originalConfirm.replaceWith(confirmButton);

    let activeSessions = true;
    const confirmed = [];

    function statusText() {
      if (activeSessions === true) return "RemoteApp session active";
      if (activeSessions === false) return "No RemoteApp session detected";
      return "RemoteApp session status is indeterminate";
    }

    function render() {
      main.innerHTML = `
        <section class="office-lifecycle-e2e" aria-label="Office lifecycle test harness">
          <h2>Office profile lifecycle</h2>
          <p data-testid="office-session-state" aria-live="polite">${statusText()}</p>
          <div class="office-lifecycle-actions">
            <button type="button" class="btn btn-ghost" data-office-lifecycle-action="start">Start</button>
            <button type="button" class="btn btn-ghost" data-office-lifecycle-action="pause">Pause</button>
            <button type="button" class="btn btn-ghost" data-office-lifecycle-action="stop">Stop</button>
            <button type="button" class="btn btn-ghost" data-office-lifecycle-action="restart">Restart</button>
          </div>
          <ol data-testid="office-lifecycle-log">
            ${confirmed.map(item => `<li>${item}</li>`).join("")}
          </ol>
        </section>`;
    }

    main.addEventListener("click", (event) => {
      const control = event.target.closest?.("[data-office-lifecycle-action]");
      if (!control) return;
      const action = control.dataset.officeLifecycleAction;
      if (["pause", "stop", "restart"].includes(action) && activeSessions !== false) {
        modal.dataset.confirmCode = "office_apps_maybe_open";
        modal.dataset.lifecycleAction = action;
        document.querySelector("#confirm-title").textContent = "Office apps may still be open";
        document.querySelector("#confirm-message").textContent =
          activeSessions === true
            ? "Active RemoteApp session detected. Confirm code office_apps_maybe_open is required."
            : "RemoteApp session status is indeterminate. Confirm code office_apps_maybe_open is required.";
        confirmButton.textContent = "Continue";
        modal.classList.add("open");
        return;
      }
      confirmed.push(`${action}:direct`);
      render();
    });

    confirmButton.addEventListener("click", () => {
      confirmed.push(`${modal.dataset.lifecycleAction}:confirmed:${modal.dataset.confirmCode}`);
      modal.classList.remove("open");
      render();
    });

    window.__winboxE2eLifecycle = {
      actions: () => [...confirmed],
      setActiveSessions(value) {
        activeSessions = value;
        render();
      },
    };
    render();
    return { ok: true };
  });
  expect(installed.ok).toBe(true);
}

function seedOfficeUpgradeFixture() {
  const profileConfigDir = resolve(xdgConfigHome, "winbox/profiles/office");
  const profileDataDir = resolve(xdgDataHome, "winbox/profiles/office");
  const applicationsDir = resolve(xdgDataHome, "applications");
  const mimeappsPath = resolve(xdgConfigHome, "mimeapps.list");

  rmSync(profileConfigDir, { recursive: true, force: true });
  rmSync(profileDataDir, { recursive: true, force: true });
  rmSync(resolve(applicationsDir, "excel-o365.desktop"), { force: true });
  rmSync(resolve(applicationsDir, "word-o365.desktop"), { force: true });
  rmSync(resolve(applicationsDir, "powerpoint-o365.desktop"), { force: true });
  rmSync(upgradeFallbackLog, { force: true });

  mkdirSync(profileConfigDir, { recursive: true });
  mkdirSync(profileDataDir, { recursive: true });
  mkdirSync(applicationsDir, { recursive: true });
  mkdirSync(dirname(mimeappsPath), { recursive: true });
  mkdirSync(shimLogDir, { recursive: true });

  writeFileSync(
    resolve(profileConfigDir, "config.env"),
    [
      "PROFILE_KIND=office",
      "VERSION=11",
      "LANGUAGE=pt-br",
      "OFFICE_LANGUAGE=pt-br",
      "OFFICE_PRODUCT_ID=O365ProPlusRetail",
      "RDP_PORT=13389",
      "CONTAINER_NAME=winbox-office",
      `SHARED_DIR=${profileDataDir}/shared`,
      "",
    ].join("\n"),
  );
  writeFileSync(resolve(profileConfigDir, "office-provisioning.json"), JSON.stringify(officeReadyState(), null, 2));

  writeFileSync(
    resolve(applicationsDir, "excel-o365.desktop"),
    desktopEntry("Excel", "excel", [
      "application/vnd.ms-excel",
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ]),
  );
  writeFileSync(
    resolve(applicationsDir, "word-o365.desktop"),
    desktopEntry("Word", "word", [
      "application/msword",
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ]),
  );
  writeFileSync(
    resolve(applicationsDir, "powerpoint-o365.desktop"),
    desktopEntry("PowerPoint", "powerpoint", [
      "application/vnd.ms-powerpoint",
      "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ]),
  );
  writeFileSync(
    mimeappsPath,
    [
      "[Default Applications]",
      "application/vnd.ms-excel=excel-o365.desktop",
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet=excel-o365.desktop",
      "application/msword=word-o365.desktop",
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document=word-o365.desktop",
      "application/vnd.ms-powerpoint=powerpoint-o365.desktop",
      "application/vnd.openxmlformats-officedocument.presentationml.presentation=powerpoint-o365.desktop",
      "",
    ].join("\n"),
  );

  return officeUpgradeSnapshot();
}

function officeUpgradeSnapshot() {
  const profileConfigDir = resolve(xdgConfigHome, "winbox/profiles/office");
  const applicationsDir = resolve(xdgDataHome, "applications");
  const mimeappsPath = resolve(xdgConfigHome, "mimeapps.list");
  return {
    state: JSON.parse(readFileSync(resolve(profileConfigDir, "office-provisioning.json"), "utf8")),
    desktop: {
      excel: readFileSync(resolve(applicationsDir, "excel-o365.desktop"), "utf8"),
      word: readFileSync(resolve(applicationsDir, "word-o365.desktop"), "utf8"),
      powerpoint: readFileSync(resolve(applicationsDir, "powerpoint-o365.desktop"), "utf8"),
    },
    mimeapps: readFileSync(mimeappsPath, "utf8"),
  };
}

function simulateUpgradeFallback(fromVersion, toVersion) {
  const markerDir = resolve(xdgDataHome, "winbox/e2e-upgrade");
  const marker = resolve(markerDir, "current-binary-version");
  mkdirSync(markerDir, { recursive: true });
  writeFileSync(marker, `${fromVersion}\n`);
  writeFileSync(marker, `${toVersion}\n`);
  writeFileSync(
    upgradeFallbackLog,
    `simulated upgrade fallback: swapped binary marker from ${fromVersion} to ${toVersion}\n`,
  );
}

function officeReadyState() {
  const phases = Object.fromEntries(
    [
      "preflight",
      "byol_acceptance",
      "profile_config",
      "windows_prepare",
      "windows_install",
      "remoteapp_prepare",
      "office_stage_odt",
      "office_install",
      "winapps_config",
      "desktop_registration",
      "file_association",
      "final_verify",
      "first_launch",
    ].map(phase => [
      phase,
      {
        status: phase === "first_launch" ? "pending" : "done",
        attempt: phase === "first_launch" ? 0 : 1,
        history: [],
      },
    ]),
  );
  return {
    schemaVersion: "1.0",
    profile: "office",
    profileKind: "office",
    status: "ready",
    createdAt: "2026-07-08T00:00:00Z",
    updatedAt: "2026-07-08T00:00:00Z",
    byol: {
      accepted: true,
      acceptedAt: "2026-07-08T00:00:00Z",
      textVersion: "2026-07-08",
    },
    options: {
      productId: "O365ProPlusRetail",
      language: "pt-br",
      officeChannel: "Current",
      windowsVersion: "11",
      winappsCommit: "5cbf7381f9a12af630e5a289d6dab5f7adc70e5d",
    },
    phases,
    managedPaths: {
      desktopFiles: [
        resolve(xdgDataHome, "applications/excel-o365.desktop"),
        resolve(xdgDataHome, "applications/word-o365.desktop"),
        resolve(xdgDataHome, "applications/powerpoint-o365.desktop"),
      ],
      mimeTypes: [
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.ms-powerpoint",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
      ],
      winappsConfOwned: true,
    },
    activeSessions: false,
  };
}

function desktopEntry(label, appId, mimeTypes) {
  const launcher = `${appId}-o365`;
  return [
    "[Desktop Entry]",
    `Name=Microsoft ${label}`,
    `Exec=winbox office launch office ${appId} --gui-progress -- %F`,
    `Icon=${launcher}`,
    `StartupWMClass=Microsoft ${label}`,
    "Categories=Office;",
    `MimeType=${mimeTypes.join(";")};`,
    "Type=Application",
    "",
  ].join("\n");
}
