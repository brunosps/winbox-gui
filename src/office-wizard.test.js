import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  bindOfficeWizard,
  canStartProvisioning,
  canContinueFromPreflight,
  initialOfficeWizardState,
  officeWizardCta,
  officeWizardReducer,
  renderAdoptionReview,
  renderByolStep,
  renderOfficeWizard,
  renderPreflightStep,
} from "./office-wizard.js";
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
  "officeWizard.preflight.title": "Pre-flight",
  "officeWizard.preflight.desc": "Check host requirements before provisioning.",
  "officeWizard.preflight.duration": "Provisioning can take around 45 minutes.",
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
  "officeWizard.status.pending": "Pending",
  "officeWizard.status.running": "Running",
  "officeWizard.status.done": "Done",
  "officeWizard.status.failed": "Failed",
  "officeWizard.status.skipped": "Skipped",
  "officeWizard.cta.back": "Back",
  "officeWizard.cta.continue": "Continue",
  "officeWizard.cta.loading": "Working",
  "officeWizard.cta.startPreflight": "Run pre-flight",
  "officeWizard.validation.profileName": "Use lowercase letters, numbers, _ or -.",
  "officeWizard.validation.byol": "Accept BYOL to continue.",
  "officeWizard.profileFor": "Office profile {name}",
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
  assert.match(html, /role="alert"/);
  assert.doesNotMatch(html, /<script/);
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

function officeWizardKeys(path) {
  return [...readFileSync(path, "utf8").matchAll(/"((?:app\.newProfile\.office)|(?:officeWizard\.[^"]+))":/g)]
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

function actionTarget(action) {
  return {
    disabled: false,
    dataset: { officeWizardAction: action },
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
