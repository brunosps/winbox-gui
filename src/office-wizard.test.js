import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  applyOfficeProgressEvent,
  bindOfficeWizard,
  canStartProvisioning,
  canContinueFromPreflight,
  initialOfficeWizardState,
  isOfficeReadyForLaunch,
  officeWizardCta,
  officeWizardReducer,
  renderActivationGuidance,
  renderAdoptionReview,
  renderByolStep,
  renderLicenseScopeStep,
  renderOfficeWizard,
  renderPreflightStep,
  renderProvisioningStep,
  renderReadyStep,
  renderTelemetryPreference,
  renderThirdPartyAttributions,
  stateFromProvisioningResponse,
} from "./office-wizard.js";
import {
  initialOfficeProgressWindowState,
  officeProgressWindowReducer,
  renderOfficeProgressWindow,
} from "./office-progress-window.js";
import { escapeAttr, escapeHtml } from "./dom-utils.js";

const dict = {
  "officeWizard.title": "Office profile",
  "officeWizard.eyebrow": "Office",
  "officeWizard.subtitle": "Install Office with Windows and WinApps.",
  "officeWizard.close": "Close",
  "officeWizard.stepsLabel": "Office wizard steps",
  "officeWizard.step.intro": "Overview",
  "officeWizard.step.profile": "Profile",
  "officeWizard.step.license": "License",
  "officeWizard.step.preflight": "Pre-flight",
  "officeWizard.step.provisioning": "Provisioning",
  "officeWizard.intro.title": "Create Office profile",
  "officeWizard.intro.desc": "Use the wizard to provision the VM.",
  "officeWizard.profile.title": "Profile settings",
  "officeWizard.profile.desc": "Pick a local profile name.",
  "officeWizard.license.title": "Bring your own license",
  "officeWizard.license.desc": "Microsoft 365 activation happens inside Windows.",
  "officeWizard.license.accept": "I will use my own license.",
  "officeWizard.byol.item.noLicense": "winbox does not provide Windows, Office, product keys or activation.",
  "officeWizard.byol.item.msMedia": "Trial or evaluation media comes from Microsoft servers when applicable.",
  "officeWizard.byol.item.genericKeys": "Generic install keys are not activation licenses.",
  "officeWizard.byol.item.technicalActivation": "Technical activation does not prove legal ownership.",
  "officeWizard.byol.item.singleUser": "This flow is scoped to one local user and one machine.",
  "officeWizard.byol.item.eula": "Provisioning may accept Microsoft terms through AcceptEULA and unattend.",
  "officeWizard.byol.error": "Accept BYOL before provisioning.",
  "officeWizard.byol.loading": "Recording acceptance.",
  "officeWizard.licenseInfo.title": "Scope and license guidance",
  "officeWizard.licenseInfo.scope": "This MVP is for one local user on the same physical machine.",
  "officeWizard.licenseInfo.noServer": "It does not cover multi-user, server or third-party remote access.",
  "officeWizard.licenseInfo.noEnforcement": "winbox does not validate plans, product keys, subscriptions or compliance.",
  "officeWizard.licenseInfo.oem": "A host OEM Windows license generally does not cover an additional VM.",
  "officeWizard.licenseInfo.m365Business": "Microsoft 365 Apps for Business is not supported for virtual desktop.",
  "officeWizard.licenseInfo.positive": "Enterprise and Business Premium are supported virtual desktop plans.",
  "officeWizard.licenseInfo.thirdPartyBoundary": "WinApps stays upstream at runtime and is not vendored.",
  "officeWizard.licenseInfo.adrLink": "Runtime boundary ADR",
  "officeWizard.thirdParty.title": "Third-party notices",
  "officeWizard.thirdParty.desc": "This flow uses dockurr/windows, WinApps, WinApps-Launcher, FreeRDP and ODT.",
  "officeWizard.thirdParty.link": "Open THIRD-PARTY.md",
  "officeWizard.telemetry.title": "Share minimal provisioning telemetry",
  "officeWizard.telemetry.desc": "Optional and off by default. Sends phase and duration only.",
  "officeWizard.preflight.title": "Pre-flight",
  "officeWizard.preflight.desc": "Check host requirements before provisioning.",
  "officeWizard.preflight.duration": "Provisioning can take around 45 minutes.",
  "officeWizard.provisioning.title": "Provisioning Office",
  "officeWizard.provisioning.desc": "Leave the computer online while winbox works through the phases.",
  "officeWizard.provisioning.durationTitle": "Duration expectation",
  "officeWizard.provisioning.duration": "This can take up to around 45 minutes and needs no interaction.",
  "officeWizard.provisioning.largeDownload": "Large downloads can take longer on slow networks.",
  "officeWizard.provisioning.failed": "Provisioning failed.",
  "officeWizard.ready.title": "Office is installed",
  "officeWizard.ready.desc": "Profile {profile} is ready. First launch opens through RemoteApp.",
  "officeWizard.ready.firstLaunch.pending": "First launch has not run yet. This does not block ready.",
  "officeWizard.ready.firstLaunch.running": "First launch is running.",
  "officeWizard.ready.firstLaunch.done": "First launch completed.",
  "officeWizard.ready.firstLaunch.failed": "First launch failed.",
  "officeWizard.ready.firstLaunch.skipped": "First launch skipped.",
  "officeWizard.activation.title": "Sign in and activation",
  "officeWizard.activation.remoteApp": "Apps open through RemoteApp from {profile} and may ask for Microsoft 365 sign-in.",
  "officeWizard.activation.userResponsibility": "Activation is the user's responsibility. winbox installs Office, not activated.",
  "officeWizard.activation.noAutomation": "winbox does not automate, bypass or validate Office activation.",
  "officeWizard.launch.actionsLabel": "Open an Office app",
  "officeWizard.launch.excel": "Open Excel",
  "officeWizard.launch.word": "Open Word",
  "officeWizard.launch.powerpoint": "Open PowerPoint",
  "officeWizard.preflight.loading": "Checking host.",
  "officeWizard.preflight.blocked": "Resolve blockers before provisioning.",
  "officeWizard.preflight.checksLabel": "Pre-flight checks",
  "officeWizard.preflight.unknownRequirement": "Unknown requirement",
  "officeWizard.preflight.noImpact": "Impact unavailable.",
  "officeWizard.preflight.status.ok": "OK",
  "officeWizard.preflight.status.warning": "Warning",
  "officeWizard.preflight.status.blocker": "Blocker",
  "officeWizard.preflight.reason.blocker": "Resolve blockers first.",
  "officeWizard.preflight.reason.warning": "Confirm warnings to continue.",
  "officeWizard.preflight.reason.adoption": "Review adoption before continuing.",
  "officeWizard.warningOverride.title": "Resource warning",
  "officeWizard.warningOverride.desc": "{count} warning(s) can reduce reliability.",
  "officeWizard.warningOverride.confirm": "Continue with these warnings.",
  "officeWizard.adoption.title": "Adopt existing setup",
  "officeWizard.adoption.desc": "Review what winbox will manage.",
  "officeWizard.adoption.managed": "Managed by winbox",
  "officeWizard.adoption.preserved": "Preserved",
  "officeWizard.adoption.confirm": "Adopt this setup with the scope shown.",
  "officeWizard.adoption.unknown": "Detected item",
  "officeWizard.adoption.none": "None",
  "officeWizard.adoption.defaultManaged": "Managed by default",
  "officeWizard.adoption.defaultPreserved": "Preserved by default",
  "officeWizard.adoption.status.compatible": "Compatible",
  "officeWizard.adoption.status.partial": "Partial",
  "officeWizard.adoption.status.unsafe": "Unsafe",
  "officeWizard.adoption.scope.profileConfig": "Profile config",
  "officeWizard.adoption.scope.winappsConf": "WinApps config",
  "officeWizard.adoption.scope.desktopEntries": "Desktop entries",
  "officeWizard.adoption.scope.fileAssociations": "File associations",
  "officeWizard.adoption.scope.diskLifecycle": "Disk lifecycle",
  "officeWizard.adoption.scope.winappsClone": "Existing WinApps clone",
  "officeWizard.field.profileName": "Profile name",
  "officeWizard.field.language": "Office language",
  "officeWizard.language.ptBr": "Portuguese (Brazil)",
  "officeWizard.language.enUs": "English (US)",
  "officeWizard.summary.profile": "Profile",
  "officeWizard.summary.office": "Office",
  "officeWizard.phasesLabel": "Provisioning phases",
  "officeWizard.phase.preflight": "Pre-flight",
  "officeWizard.phase.byol_acceptance": "License",
  "officeWizard.phase.profile_config": "Profile config",
  "officeWizard.phase.windows_prepare": "Windows prepare",
  "officeWizard.phase.windows_install": "Windows install",
  "officeWizard.phase.remoteapp_prepare": "RemoteApp prepare",
  "officeWizard.phase.office_stage_odt": "Stage ODT",
  "officeWizard.phase.office_install": "Install Office",
  "officeWizard.phase.winapps_config": "Configure WinApps",
  "officeWizard.phase.desktop_registration": "Desktop registration",
  "officeWizard.phase.file_association": "File associations",
  "officeWizard.phase.final_verify": "Final verification",
  "officeWizard.phase.first_launch": "First launch",
  "officeWizard.status.pending": "Pending",
  "officeWizard.status.running": "Running",
  "officeWizard.status.done": "Done",
  "officeWizard.status.failed": "Failed",
  "officeWizard.status.skipped": "Skipped",
  "officeWizard.cta.back": "Back",
  "officeWizard.cta.continue": "Continue",
  "officeWizard.cta.loading": "Working",
  "officeWizard.cta.startPreflight": "Run pre-flight",
  "officeWizard.cta.continueProvisioning": "Continue",
  "officeWizard.cta.provisioning": "Provisioning",
  "officeWizard.validation.profileName": "Use lowercase letters, numbers, _ or -.",
  "officeWizard.validation.byol": "Accept BYOL to continue.",
  "officeWizard.profileFor": "Office profile {name}",
  "officeProgress.eyebrow": "Office",
  "officeProgress.title": "Office is starting",
  "officeProgress.subtitle": "Opening {app} from {profile}.",
  "officeProgress.officeApp": "Office app",
  "officeProgress.waiting": "Waiting for progress.",
  "officeProgress.closeHint": "This window can stay open while the app starts.",
};

const deps = {
  t: (key, vars = {}) => {
    let value = dict[key] || key;
    for (const [name, replacement] of Object.entries(vars)) {
      value = value.replaceAll(`{${name}}`, String(replacement));
    }
    return value;
  },
  escapeHtml,
  escapeAttr,
};

test("office_wizard_shell_escapes_dynamic_profile_data", () => {
  const html = renderOfficeWizard(
    initialOfficeWizardState({
      profileName: `office"><img src=x onerror=alert(1)>`,
    }),
    deps,
  );

  assert.match(html, /office&quot;&gt;&lt;img src=x onerror=alert\(1\)&gt;/);
  assert.doesNotMatch(html, /<img src=x onerror=/);
});

test("office_wizard_reducer_updates_fields_and_phase_status", () => {
  const changed = officeWizardReducer(initialOfficeWizardState(), {
    type: "field",
    field: "profileName",
    value: "office-work",
  });
  assert.equal(changed.profileName, "office-work");

  const phase = officeWizardReducer(changed, {
    type: "phase_status",
    phase: "preflight",
    status: "done",
  });
  assert.equal(phase.phaseStatuses.preflight, "done");

  const next = officeWizardReducer(phase, { type: "next" });
  assert.equal(next.step, "profile");
});

test("office_wizard_cta_disables_invalid_profile_and_byol_gate", () => {
  const invalidProfile = officeWizardCta(
    initialOfficeWizardState({ step: "profile", profileName: "Office!" }),
    deps,
  );
  assert.equal(invalidProfile.disabled, true);
  assert.equal(invalidProfile.reason, dict["officeWizard.validation.profileName"]);

  const byolBlocked = officeWizardCta(
    initialOfficeWizardState({ step: "license", byolAccepted: false }),
    deps,
  );
  assert.equal(byolBlocked.disabled, true);
  assert.equal(byolBlocked.reason, dict["officeWizard.validation.byol"]);

  const preflight = officeWizardCta(
    initialOfficeWizardState({ step: "preflight", byolAccepted: true }),
    deps,
  );
  assert.equal(preflight.disabled, false);
  assert.equal(preflight.action, "start");
});

test("byol_acceptance_blocks_side_effects_until_checked", () => {
  const blocked = initialOfficeWizardState({ step: "preflight", byolAccepted: false });
  assert.equal(canStartProvisioning(blocked), false);
  assert.equal(officeWizardCta(blocked, deps).disabled, true);

  const root = fakeRoot({ byolChecked: false });
  const actions = [];
  let starts = 0;
  bindOfficeWizard(root, {
    dispatch: action => actions.push(action),
    onStart: () => {
      starts += 1;
    },
  });
  root.fire("click", fakeEvent(actionTarget("start")));

  assert.equal(starts, 0);
  assert.deepEqual(actions, [
    { type: "set_status", status: "error", errorCode: "byol_not_accepted" },
  ]);

  const acceptedRoot = fakeRoot({ byolChecked: true });
  bindOfficeWizard(acceptedRoot, {
    dispatch: action => actions.push(action),
    onStart: () => {
      starts += 1;
    },
  });
  acceptedRoot.fire("click", fakeEvent(actionTarget("start")));
  assert.equal(starts, 1);
});

test("renderByolStep_shows_required_disclaimer_and_alert_states", () => {
  const html = renderByolStep(
    initialOfficeWizardState({
      step: "license",
      errorCode: "byol_not_accepted",
    }),
    deps,
  );

  assert.match(html, /does not provide Windows, Office, product keys or activation/);
  assert.match(html, /Generic install keys are not activation licenses/);
  assert.match(html, /Technical activation does not prove legal ownership/);
  assert.match(html, /AcceptEULA and unattend/);
  assert.match(html, /Scope and license guidance/);
  assert.match(html, /one local user on the same physical machine/);
  assert.match(html, /role="alert"/);
  assert.doesNotMatch(html, /<script/);
});

test("license_guidance_has_no_programmatic_enforcement_branch", () => {
  const state = initialOfficeWizardState({
    step: "license",
    byolAccepted: true,
    licensePlan: "m365_apps_for_business",
    windowsLicense: "oem",
  });
  const html = renderLicenseScopeStep(state, deps);
  const cta = officeWizardCta(state, deps);

  assert.equal(canStartProvisioning(state), true);
  assert.equal(cta.disabled, false);
  assert.match(html, /does not validate plans, product keys, subscriptions or compliance/);
  assert.match(html, /Apps for Business is not supported/);
  assert.match(html, /OEM Windows license generally does not cover/);
});

test("single_user_scope_copy_is_localized", () => {
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");
  const html = renderLicenseScopeStep(initialOfficeWizardState(), deps);

  assert.match(html, /one local user on the same physical machine/);
  assert.match(html, /multi-user, server or third-party remote access/);
  assert.equal(en.includes("officeWizard.licenseInfo.scope"), true);
  assert.equal(pt.includes("officeWizard.licenseInfo.scope"), true);
  assert.equal(en.includes("officeWizard.licenseInfo.noServer"), true);
  assert.equal(pt.includes("officeWizard.licenseInfo.noServer"), true);
});

test("license_guidance_escapes_dynamic_links", () => {
  const html = renderLicenseScopeStep(initialOfficeWizardState(), {
    ...deps,
    licenseAdrHref: `javascript:alert(1)" onclick="alert(2)`,
  });

  assert.match(html, /href="#"/);
  assert.doesNotMatch(html, /javascript:alert/);
  assert.doesNotMatch(html, /onclick=/);
});

test("third_party_attribution_surface_is_localized", () => {
  const html = renderThirdPartyAttributions({
    ...deps,
    thirdPartyHref: `javascript:alert(1)" onclick="alert(2)`,
  });
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");
  const notices = readFileSync("THIRD-PARTY.md", "utf8");

  assert.match(html, /Third-party notices/);
  assert.match(html, /dockurr\/windows, WinApps, WinApps-Launcher, FreeRDP/);
  assert.match(html, /href="#"/);
  assert.doesNotMatch(html, /javascript:alert/);
  assert.equal(en.includes("officeWizard.thirdParty.title"), true);
  assert.equal(pt.includes("officeWizard.thirdParty.title"), true);
  assert.equal(en.includes("officeWizard.thirdParty.link"), true);
  assert.equal(pt.includes("officeWizard.thirdParty.link"), true);
  assert.match(notices, /dockurr\/windows \| MIT/);
  assert.match(notices, /WinApps \| AGPL-3\.0/);
  assert.match(notices, /WinApps-Launcher \| GPL-3\.0/);
  assert.match(notices, /FreeRDP \| Apache-2\.0/);
  assert.match(notices, /libfuse2t64/);
});

test("telemetry_preference_defaults_off_and_is_localized", () => {
  const state = initialOfficeWizardState();
  const html = renderTelemetryPreference(state, deps);
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");

  assert.equal(state.telemetryOptIn, false);
  assert.match(html, /data-office-wizard-field="telemetryOptIn"/);
  assert.doesNotMatch(html, /checked/);
  assert.match(html, /Optional and off by default/);
  assert.equal(en.includes("officeWizard.telemetry.title"), true);
  assert.equal(pt.includes("officeWizard.telemetry.title"), true);
});

test("tauri_bundle_targets_are_deb_and_appimage", () => {
  const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));

  assert.deepEqual(config.bundle.targets, ["deb", "appimage"]);
  assert.equal(JSON.stringify(config.plugins || {}).includes("updater"), false);
});

test("release_workflow_generates_sha256sums", () => {
  const workflow = readFileSync(".github/workflows/release-cli.yml", "utf8");

  assert.match(workflow, /runs-on: ubuntu-22\.04/);
  assert.match(workflow, /cargo tauri build --bundles deb,appimage/);
  assert.match(workflow, /sha256sum "\$\{files\[@\]\}" > SHA256SUMS/);
  assert.match(workflow, /src-tauri\/target\/release\/bundle\/deb\/\*\.deb/);
  assert.match(workflow, /src-tauri\/target\/release\/bundle\/appimage\/\*\.AppImage/);
  assert.doesNotMatch(workflow, /docker-ce|freerdp3-x11|flatpak install/i);
});

test("office_coverage_jobs_measure_new_modules", () => {
  const workflow = readFileSync(".github/workflows/ci.yml", "utf8");

  assert.match(workflow, /cargo install cargo-llvm-cov --locked/);
  assert.match(workflow, /cargo llvm-cov --manifest-path src-tauri\/Cargo\.toml --all-targets/);
  assert.match(workflow, /src-tauri\/src\/core\/office_state\.rs/);
  assert.match(workflow, /src-tauri\/src\/core\/office_odt\.rs/);
  assert.match(workflow, /src-tauri\/src\/core\/office_preflight\.rs/);
  assert.match(workflow, /src-tauri\/src\/commands\/office\.rs/);
  assert.match(workflow, /node --test --experimental-test-coverage src\/office-wizard\.test\.js/);
});

test("office_beta_release_gate_includes_upgrade_scenario", () => {
  const checklist = readFileSync("tests/e2e/office-beta-readiness.md", "utf8");

  assert.match(checklist, /office_real_vm_beta_readiness/);
  assert.match(checklist, /FR-1\.4/);
  assert.match(checklist, /FR-8\.4/);
  assert.match(checklist, /3 provisionamentos limpos/);
  assert.match(checklist, /2 adoc(?:o|õ)es/);
  assert.match(checklist, /upgrade preservado/);
  assert.match(checklist, /Coolify/);
});

test("preflight_blocker_renders_action_hint_escaped", () => {
  const state = initialOfficeWizardState({
    step: "preflight",
    byolAccepted: true,
    preflightChecks: [
      {
        id: "preflight_kvm_missing",
        status: "blocker",
        requirement: "KVM <required>",
        impact: "Office VM cannot boot <script>alert(1)</script>",
        action_hint: "Run sudo usermod -aG kvm $USER && reboot <b>now</b>",
      },
    ],
  });

  const html = renderPreflightStep(state, deps);

  assert.match(html, /role="alert"/);
  assert.match(html, /KVM &lt;required&gt;/);
  assert.match(html, /Office VM cannot boot &lt;script&gt;alert\(1\)&lt;\/script&gt;/);
  assert.match(html, /&lt;b&gt;now&lt;\/b&gt;/);
  assert.doesNotMatch(html, /<script>alert/);
});

test("resource_warning_requires_ui_override", () => {
  const warningState = initialOfficeWizardState({
    step: "preflight",
    byolAccepted: true,
    preflightChecks: [
      {
        id: "resource_ram_recommended",
        status: "warning",
        requirement: "8 GB RAM recommended",
        impact: "Office install can be slow.",
        action_hint: "Increase RAM or confirm override.",
      },
    ],
  });

  assert.equal(canContinueFromPreflight(warningState), false);
  assert.equal(officeWizardCta(warningState, deps).disabled, true);
  assert.match(renderPreflightStep(warningState, deps), /data-office-wizard-field="warningOverride"/);

  const accepted = officeWizardReducer(warningState, {
    type: "field",
    field: "warningOverride",
    value: true,
  });
  assert.equal(canContinueFromPreflight(accepted), true);
  assert.equal(officeWizardCta(accepted, deps).disabled, false);
});

test("adoption_review_requires_explicit_choice", () => {
  const state = initialOfficeWizardState({
    step: "preflight",
    byolAccepted: true,
    preflightChecks: [],
    adoptionCandidates: [
      {
        id: "winapps_conf",
        kind: "winapps_conf",
        status: "compatible",
        evidence: "~/.config/winapps/winapps.conf <kept>",
        managedByDefault: false,
      },
    ],
    managedScope: {
      manageProfileConfig: true,
      manageWinAppsConf: false,
      manageDesktopEntries: true,
      manageFileAssociations: true,
      manageDiskLifecycle: false,
      preserveExistingWinAppsClone: true,
    },
  });

  const html = renderAdoptionReview(state, deps);

  assert.equal(canContinueFromPreflight(state), false);
  assert.match(html, /data-office-wizard-field="adoptionConfirmed"/);
  assert.match(html, /~\/.config\/winapps\/winapps.conf &lt;kept&gt;/);
  assert.match(html, /Profile config/);
  assert.match(html, /WinApps config/);

  const confirmed = officeWizardReducer(state, {
    type: "field",
    field: "adoptionConfirmed",
    value: true,
  });
  assert.equal(canContinueFromPreflight(confirmed), true);
});

test("provisioning_intro_shows_duration_expectation", () => {
  const html = renderProvisioningStep(
    initialOfficeWizardState({ step: "provisioning" }),
    deps,
  );

  assert.match(html, /up to around 45 minutes/);
  assert.match(html, /needs no interaction/);
  assert.match(html, /Large downloads/);
});

test("provisioning_progress_uses_aria_live", () => {
  const state = applyOfficeProgressEvent(
    initialOfficeWizardState({ profileName: "office", step: "provisioning" }),
    {
      type: "operation-progress",
      profile: "office",
      op: "office_provision",
      step: "office_odt_install",
      status: "running",
      message: "Downloading Office payload <slow>",
      timestamp: "2026-07-08T12:00:00Z",
    },
  );
  const html = renderProvisioningStep(state, deps);

  assert.equal(state.phaseStatuses.office_install, "running");
  assert.match(html, /aria-live="polite"/);
  assert.match(html, /Downloading Office payload &lt;slow&gt;/);
  assert.doesNotMatch(html, /<slow>/);
});

test("wizard_resume_renders_persisted_failed_phase", () => {
  const resumed = stateFromProvisioningResponse(initialOfficeWizardState(), {
    status: "failed",
    phases: {
      preflight: { status: "done" },
      office_install: { status: "failed" },
      final_verify: { status: "pending" },
    },
    lastError: {
      code: "guest_phase_timeout",
      message: "Office marker did not appear.",
      phase: "office_install",
      retryable: true,
    },
  });

  assert.equal(resumed.step, "provisioning");
  assert.equal(resumed.phaseStatuses.office_install, "failed");
  assert.equal(resumed.errorCode, "guest_phase_timeout");
  assert.match(renderProvisioningStep(resumed, deps), /Office marker did not appear/);
});

test("retry_ui_repaints_invalidated_descendants", () => {
  const afterRetry = stateFromProvisioningResponse(
    initialOfficeWizardState({
      step: "provisioning",
      phaseStatuses: {
        winapps_config: "done",
        desktop_registration: "done",
        final_verify: "done",
      },
    }),
    {
      status: "running",
      phases: {
        winapps_config: { status: "running" },
        desktop_registration: { status: "pending" },
        file_association: { status: "pending" },
        final_verify: { status: "pending" },
      },
    },
  );

  assert.equal(afterRetry.phaseStatuses.winapps_config, "running");
  assert.equal(afterRetry.phaseStatuses.desktop_registration, "pending");
  assert.equal(afterRetry.phaseStatuses.final_verify, "pending");
});

test("office_progress_window_renders_same_operation_events", () => {
  const state = officeProgressWindowReducer(
    initialOfficeProgressWindowState({ profileName: "office", appId: "excel" }),
    {
      type: "operation_progress",
      event: {
        type: "operation-progress",
        profile: "office",
        op: "office_launch",
        step: "office_cold_start",
        status: "running",
        message: "Starting VM before Excel",
      },
    },
  );
  const html = renderOfficeProgressWindow(state, deps);

  assert.equal(state.wizardState.phaseStatuses.first_launch, "running");
  assert.match(html, /Starting VM before Excel/);
  assert.match(html, /aria-live="polite"/);
});

test("first_launch_is_informational_not_ready_gate", () => {
  const state = initialOfficeWizardState({
    step: "provisioning",
    status: "success",
    phaseStatuses: {
      final_verify: "done",
      first_launch: "pending",
    },
  });
  const html = renderReadyStep(state, deps);

  assert.equal(isOfficeReadyForLaunch(state), true);
  assert.match(html, /Office is installed/);
  assert.match(html, /This does not block ready/);
  assert.match(html, /data-office-wizard-action="launch-app"/);
});

test("activation_guidance_is_static_and_localized", () => {
  const state = initialOfficeWizardState({
    profileName: `office"><script>alert(1)</script>`,
  });
  const html = renderActivationGuidance(state, deps);
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");

  assert.match(html, /Microsoft 365 sign-in/);
  assert.match(html, /Activation is the user&#39;s responsibility/);
  assert.match(html, /does not automate, bypass or validate Office activation/);
  assert.match(html, /office&quot;&gt;&lt;script&gt;alert\(1\)&lt;\/script&gt;/);
  assert.doesNotMatch(html, /heuristic|detect|validate activation/i);
  assert.equal(en.includes("officeWizard.activation.noAutomation"), true);
  assert.equal(pt.includes("officeWizard.activation.noAutomation"), true);
});

test("post_launch_actions_call_office_launcher", () => {
  const root = fakeRoot();
  const launches = [];
  bindOfficeWizard(root, {
    getState: () => initialOfficeWizardState({ profileName: "office" }),
    onLaunchApp: args => launches.push(args),
  });

  root.fire("click", fakeEvent(actionTarget("launch-app", { officeApp: "excel" })));

  assert.deepEqual(launches, [
    {
      name: "office",
      appId: "excel",
      files: [],
      guiProgress: true,
    },
  ]);
});

test("office_wizard_initial_state_is_keyboard_reachable", () => {
  const html = renderOfficeWizard(initialOfficeWizardState(), deps);

  assert.match(html, /<button type="button" class="btn btn-ghost" data-office-wizard-action="close">/);
  assert.match(html, /<button type="button" class="office-wizard-step"[^>]+aria-current="step">/);
  assert.match(html, /role="region" aria-live="polite"/);
  assert.match(html, /data-office-wizard-action="next"/);
  assert.doesNotMatch(html, /tabindex="-1"/);
});

test("office_wizard_bind_dispatches_controls_and_tears_down", () => {
  const root = fakeRoot();
  const actions = [];
  let closed = false;
  const teardown = bindOfficeWizard(root, {
    dispatch: action => actions.push(action),
    onClose: () => {
      closed = true;
    },
  });

  root.fire("input", fakeEvent(fieldTarget("profileName", "office2")));
  root.fire("click", fakeEvent(actionTarget("next")));
  root.fire("keydown", { key: "Escape" });

  assert.deepEqual(actions, [
    { type: "field", field: "profileName", value: "office2" },
    { type: "next" },
  ]);
  assert.equal(closed, true);

  teardown();
  root.fire("click", fakeEvent(actionTarget("back")));
  assert.equal(actions.length, 2);
});

test("office_wizard_locale_keys_are_parallel", () => {
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");

  assert.deepEqual(pt, en);
});

test("byol_disclaimer_i18n_parallel", () => {
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");
  const required = [
    "officeWizard.byol.item.noLicense",
    "officeWizard.byol.item.msMedia",
    "officeWizard.byol.item.genericKeys",
    "officeWizard.byol.item.technicalActivation",
    "officeWizard.byol.item.singleUser",
    "officeWizard.byol.item.eula",
    "officeWizard.byol.error",
    "officeWizard.licenseInfo.scope",
    "officeWizard.licenseInfo.noEnforcement",
    "officeWizard.licenseInfo.oem",
    "officeWizard.licenseInfo.m365Business",
    "officeWizard.licenseInfo.positive",
    "officeWizard.licenseInfo.thirdPartyBoundary",
    "officeWizard.telemetry.title",
    "officeWizard.telemetry.desc",
  ];

  for (const key of required) {
    assert.equal(en.includes(key), true, `${key} missing in en-US`);
    assert.equal(pt.includes(key), true, `${key} missing in pt-BR`);
  }
});

test("preflight_and_adoption_i18n_parallel", () => {
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");
  const required = [
    "officeWizard.preflight.checksLabel",
    "officeWizard.preflight.reason.warning",
    "officeWizard.warningOverride.confirm",
    "officeWizard.adoption.title",
    "officeWizard.adoption.confirm",
    "officeWizard.adoption.scope.winappsConf",
  ];

  for (const key of required) {
    assert.equal(en.includes(key), true, `${key} missing in en-US`);
    assert.equal(pt.includes(key), true, `${key} missing in pt-BR`);
  }
});

test("provisioning_and_progress_window_i18n_parallel", () => {
  const en = officeWizardKeys("src/locales/en-US.js");
  const pt = officeWizardKeys("src/locales/pt-BR.js");
  const required = [
    "officeWizard.step.provisioning",
    "officeWizard.provisioning.duration",
    "officeWizard.provisioning.largeDownload",
    "officeWizard.ready.title",
    "officeWizard.ready.firstLaunch.pending",
    "officeWizard.activation.remoteApp",
    "officeWizard.activation.noAutomation",
    "officeWizard.launch.excel",
    "officeWizard.phase.office_install",
    "officeWizard.phase.final_verify",
    "officeProgress.title",
    "officeProgress.subtitle",
  ];

  for (const key of required) {
    assert.equal(en.includes(key), true, `${key} missing in en-US`);
    assert.equal(pt.includes(key), true, `${key} missing in pt-BR`);
  }
});

function officeWizardKeys(path) {
  return [...readFileSync(path, "utf8").matchAll(/"((?:app\.newProfile\.office)|(?:officeWizard\.[^"]+)|(?:officeProgress\.[^"]+))":/g)]
    .map(match => match[1])
    .sort();
}

function fakeRoot(options = {}) {
  const listeners = new Map();
  return {
    addEventListener(type, fn) {
      listeners.set(type, fn);
    },
    removeEventListener(type, fn) {
      if (listeners.get(type) === fn) listeners.delete(type);
    },
    fire(type, event) {
      listeners.get(type)?.(event);
    },
    querySelector(selector) {
      if (selector !== '[data-office-wizard-field="byolAccepted"]') return null;
      return { checked: Boolean(options.byolChecked) };
    },
  };
}

function fakeEvent(target) {
  return { target };
}

function actionTarget(action, dataset = {}) {
  return {
    disabled: false,
    dataset: { officeWizardAction: action, ...dataset },
    closest(selector) {
      return selector === "[data-office-wizard-action]" ? this : null;
    },
  };
}

function fieldTarget(field, value) {
  return {
    type: "text",
    value,
    dataset: { officeWizardField: field },
    closest(selector) {
      return selector === "[data-office-wizard-field]" ? this : null;
    },
  };
}
