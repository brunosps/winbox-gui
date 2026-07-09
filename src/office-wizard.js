// Pure helpers for the Office profile wizard. Runtime dependencies stay
// injected so node:test can exercise state and rendering without Tauri.

const DEFAULT_PHASE_STATUSES = {
  preflight: "pending",
  byol_acceptance: "pending",
  profile_config: "pending",
  windows_prepare: "pending",
  windows_install: "pending",
  remoteapp_prepare: "pending",
  office_stage_odt: "pending",
  office_install: "pending",
  winapps_config: "pending",
  desktop_registration: "pending",
  file_association: "pending",
  final_verify: "pending",
  first_launch: "pending",
};

export const OFFICE_WIZARD_STEPS = ["intro", "profile", "license", "preflight", "provisioning"];

export const OFFICE_WIZARD_PHASES = Object.keys(DEFAULT_PHASE_STATUSES);

export const OFFICE_LAUNCH_APPS = [
  ["excel", "officeWizard.launch.excel"],
  ["word", "officeWizard.launch.word"],
  ["powerpoint", "officeWizard.launch.powerpoint"],
];

const PROGRESS_STEP_PHASE = {
  office_preflight: "preflight",
  office_byol: "byol_acceptance",
  office_profile_config: "profile_config",
  office_windows_prepare: "windows_prepare",
  office_windows_install: "windows_install",
  office_rdp_wait: "remoteapp_prepare",
  office_remoteapp_prepare: "remoteapp_prepare",
  office_odt_stage: "office_stage_odt",
  office_odt_install: "office_install",
  office_verify_install: "office_install",
  office_winapps_clone: "winapps_config",
  office_winapps_setup: "winapps_config",
  office_desktop_register: "desktop_registration",
  office_file_association: "file_association",
  office_final_verify: "final_verify",
  office_cold_start: "first_launch",
  office_launch_remoteapp: "first_launch",
};

const BYOL_DISCLAIMER_KEYS = [
  "officeWizard.byol.item.noLicense",
  "officeWizard.byol.item.msMedia",
  "officeWizard.byol.item.genericKeys",
  "officeWizard.byol.item.technicalActivation",
  "officeWizard.byol.item.singleUser",
  "officeWizard.byol.item.eula",
];

const LICENSE_SCOPE_KEYS = [
  "officeWizard.licenseInfo.scope",
  "officeWizard.licenseInfo.noServer",
  "officeWizard.licenseInfo.noEnforcement",
  "officeWizard.licenseInfo.oem",
  "officeWizard.licenseInfo.m365Business",
  "officeWizard.licenseInfo.positive",
  "officeWizard.licenseInfo.thirdPartyBoundary",
];

const MANAGED_SCOPE_KEYS = [
  ["manageProfileConfig", "officeWizard.adoption.scope.profileConfig"],
  ["manageWinAppsConf", "officeWizard.adoption.scope.winappsConf"],
  ["manageDesktopEntries", "officeWizard.adoption.scope.desktopEntries"],
  ["manageFileAssociations", "officeWizard.adoption.scope.fileAssociations"],
  ["manageDiskLifecycle", "officeWizard.adoption.scope.diskLifecycle"],
];

export function initialOfficeWizardState(overrides = {}) {
  return {
    step: "intro",
    profileName: "office",
    productId: "O365ProPlusRetail",
    language: "pt-br",
    byolAccepted: false,
    telemetryOptIn: false,
    warningOverride: false,
    adoptionConfirmed: false,
    preflightChecks: [],
    adoptionCandidates: [],
    managedScope: {},
    lastProgress: null,
    lastError: null,
    status: "default",
    errorCode: "",
    phaseStatuses: { ...DEFAULT_PHASE_STATUSES },
    ...overrides,
    phaseStatuses: {
      ...DEFAULT_PHASE_STATUSES,
      ...(overrides.phaseStatuses || {}),
    },
    managedScope: {
      ...(overrides.managedScope || {}),
    },
  };
}

export function officeWizardReducer(state, action = {}) {
  const current = initialOfficeWizardState(state);
  if (action.type === "field") {
    if (action.field === "byolAccepted") {
      return {
        ...current,
        byolAccepted: Boolean(action.value),
        errorCode: action.value ? "" : current.errorCode,
        status: action.value && current.errorCode === "byol_not_accepted" ? "default" : current.status,
        phaseStatuses: {
          ...current.phaseStatuses,
          byol_acceptance: action.value ? "done" : "pending",
        },
      };
    }
    return { ...current, [action.field]: action.value };
  }
  if (action.type === "set_preflight_result") {
    const result = action.result || {};
    const checks = Array.isArray(result.checks) ? result.checks : [];
    return {
      ...current,
      status: "success",
      errorCode: "",
      preflightChecks: checks,
      adoptionCandidates: Array.isArray(result.adoptionCandidates)
        ? result.adoptionCandidates
        : [],
      managedScope: result.managedScope || current.managedScope,
      warningOverride: false,
      adoptionConfirmed: false,
      phaseStatuses: {
        ...current.phaseStatuses,
        preflight: result.blockers > 0 || checks.some(check => preflightStatus(check) === "blocker") ? "failed" : "done",
      },
    };
  }
  if (action.type === "next") {
    if (current.step === "license" && !canStartProvisioning(current)) {
      return byolNotAcceptedState(current);
    }
    const index = OFFICE_WIZARD_STEPS.indexOf(current.step);
    const next = OFFICE_WIZARD_STEPS[Math.min(index + 1, OFFICE_WIZARD_STEPS.length - 1)];
    return { ...current, step: next };
  }
  if (action.type === "back") {
    const index = OFFICE_WIZARD_STEPS.indexOf(current.step);
    const next = OFFICE_WIZARD_STEPS[Math.max(index - 1, 0)];
    return { ...current, step: next };
  }
  if (action.type === "set_step" && OFFICE_WIZARD_STEPS.includes(action.step)) {
    return { ...current, step: action.step };
  }
  if (action.type === "phase_status" && OFFICE_WIZARD_PHASES.includes(action.phase)) {
    return {
      ...current,
      phaseStatuses: {
        ...current.phaseStatuses,
        [action.phase]: action.status || "pending",
      },
    };
  }
  if (action.type === "set_status") {
    return { ...current, status: action.status || "default", errorCode: action.errorCode || "" };
  }
  if (action.type === "operation_progress") {
    return applyOfficeProgressEvent(current, action.event || {});
  }
  if (action.type === "provisioning_state") {
    return stateFromProvisioningResponse(current, action.response || {});
  }
  return current;
}

export function canStartProvisioning(state) {
  return initialOfficeWizardState(state).byolAccepted === true;
}

export function canContinueFromPreflight(state) {
  const current = initialOfficeWizardState(state);
  const checks = preflightChecks(current);
  if (checks.some(check => preflightStatus(check) === "blocker")) return false;
  if (checks.some(check => preflightStatus(check) === "warning") && !current.warningOverride) {
    return false;
  }
  if (adoptionCandidates(current).length > 0 && !current.adoptionConfirmed) return false;
  return true;
}

export function isOfficeReadyForLaunch(state) {
  const current = initialOfficeWizardState(state);
  return current.phaseStatuses.final_verify === "done" || current.status === "success";
}

export function officeWizardPhaseLabel(phase, { t }) {
  const key = `officeWizard.phase.${phase}`;
  const label = t(key);
  return label === key ? phase : label;
}

export function officeWizardCta(state, { t }) {
  const current = initialOfficeWizardState(state);
  const profileName = String(current.profileName || "").trim();
  if (current.status === "loading") {
    return { action: "wait", label: t("officeWizard.cta.loading"), disabled: true };
  }
  if (current.step === "provisioning") {
    return { action: "wait", label: t("officeWizard.cta.provisioning"), disabled: true };
  }
  if (current.step === "profile" && !/^[a-z0-9][a-z0-9_-]*$/.test(profileName)) {
    return {
      action: "next",
      label: t("officeWizard.cta.continue"),
      disabled: true,
      reason: t("officeWizard.validation.profileName"),
    };
  }
  if (current.step === "license" && !current.byolAccepted) {
    return {
      action: "next",
      label: t("officeWizard.cta.continue"),
      disabled: true,
      reason: t("officeWizard.validation.byol"),
    };
  }
  if (current.step === "preflight") {
    if (!canStartProvisioning(current)) {
      return {
        action: "start",
        label: t("officeWizard.cta.startPreflight"),
        disabled: true,
        reason: t("officeWizard.validation.byol"),
      };
    }
    const reason = preflightBlockReason(current, { t });
    if (reason) {
      return {
        action: "start",
        label: t(preflightHasResult(current) ? "officeWizard.cta.continueProvisioning" : "officeWizard.cta.startPreflight"),
        disabled: true,
        reason,
      };
    }
    return {
      action: "start",
      label: t(preflightHasResult(current) ? "officeWizard.cta.continueProvisioning" : "officeWizard.cta.startPreflight"),
      disabled: false,
    };
  }
  return { action: "next", label: t("officeWizard.cta.continue"), disabled: false };
}

export function applyOfficeProgressEvent(state, event = {}) {
  const current = initialOfficeWizardState(state);
  const profile = String(event.profile || "");
  if (profile && profile !== String(current.profileName || "")) return current;
  const phase = PROGRESS_STEP_PHASE[String(event.step || "")];
  const normalizedStatus = progressStatusToPhaseStatus(event.status);
  const next = {
    ...current,
    step: "provisioning",
    status: normalizedStatus === "failed" ? "error" : "loading",
    errorCode: normalizedStatus === "failed" ? String(event.code || "office_progress_failed") : "",
    lastProgress: {
      type: String(event.type || "operation-progress"),
      profile,
      operation: String(event.operation || event.op || ""),
      phase: phase || "",
      step: String(event.step || ""),
      status: String(event.status || ""),
      message: String(event.message || ""),
      timestamp: String(event.timestamp || ""),
    },
  };
  if (!phase) return next;
  return {
    ...next,
    phaseStatuses: {
      ...next.phaseStatuses,
      [phase]: normalizedStatus,
    },
    lastError: normalizedStatus === "failed"
      ? {
          code: String(event.code || "office_progress_failed"),
          phase,
          message: String(event.message || ""),
        }
      : next.lastError,
  };
}

export function stateFromProvisioningResponse(state, response = {}) {
  const current = initialOfficeWizardState(state);
  const phases = normalizePhaseStatuses(response.phases);
  const status = String(response.status || current.status || "");
  const lastError = response.lastError || response.last_error || null;
  return {
    ...current,
    step: hasStartedProvisioning(phases, status, lastError) ? "provisioning" : current.step,
    status: lastError ? "error" : status === "ready" ? "success" : current.status,
    errorCode: lastError?.code || current.errorCode || "",
    lastError,
    phaseStatuses: {
      ...current.phaseStatuses,
      ...phases,
    },
  };
}

export function renderOfficeWizard(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const current = initialOfficeWizardState(state);
  const cta = officeWizardCta(current, { t });
  const stepIndex = OFFICE_WIZARD_STEPS.indexOf(current.step);
  const profileName = String(current.profileName || "");
  const disabledAttr = cta.disabled ? " disabled aria-disabled=\"true\"" : "";
  const describedByAttr = cta.reason ? " aria-describedby=\"office-wizard-cta-hint\"" : "";
  const ctaReason = cta.reason
    ? `<p class="office-wizard-hint" id="office-wizard-cta-hint">${escapeHtml(cta.reason)}</p>`
    : "";

  return `
    <section class="office-wizard" aria-label="${escapeAttr(t("officeWizard.title"))}">
      <div class="office-wizard-header">
        <div>
          <span class="eyebrow">${escapeHtml(t("officeWizard.eyebrow"))}</span>
          <h2>${escapeHtml(t("officeWizard.title"))}</h2>
          <p>${escapeHtml(t("officeWizard.subtitle"))}</p>
        </div>
        <button type="button" class="btn btn-ghost" data-office-wizard-action="close">
          ${escapeHtml(t("officeWizard.close"))}
        </button>
      </div>

      <div class="office-wizard-grid">
        <nav class="office-wizard-steps" aria-label="${escapeAttr(t("officeWizard.stepsLabel"))}">
          ${OFFICE_WIZARD_STEPS.map((step, index) => renderStepItem(step, index, stepIndex, deps)).join("")}
        </nav>

        <div class="office-wizard-panel" role="region" aria-live="polite" aria-labelledby="office-wizard-current-title">
          ${renderCurrentStep(current, deps)}
          ${renderPhaseList(current, deps)}
          <div class="office-wizard-actions">
            <button type="button" class="btn btn-ghost" data-office-wizard-action="back" ${stepIndex === 0 ? "disabled aria-disabled=\"true\"" : ""}>
              ${escapeHtml(t("officeWizard.cta.back"))}
            </button>
            <div class="office-wizard-action-main">
              ${ctaReason}
              <button type="button" class="btn btn-primary" data-office-wizard-action="${escapeAttr(cta.action)}"${disabledAttr}${describedByAttr}>
                ${escapeHtml(cta.label)}
              </button>
            </div>
          </div>
        </div>
      </div>
      <span class="sr-only">${escapeHtml(t("officeWizard.profileFor", { name: profileName }))}</span>
    </section>`;
}

export function bindOfficeWizard(root, deps = {}) {
  const dispatch = deps.dispatch || (() => {});
  const onClose = deps.onClose || (() => {});
  const onStart = deps.onStart || (() => {});
  const onLaunchApp = deps.onLaunchApp || (() => {});
  const getState = deps.getState || (() => deps.state || stateFromControls(root));

  const onInput = (event) => {
    const field = event.target.closest?.("[data-office-wizard-field]");
    if (!field) return;
    dispatch({
      type: "field",
      field: field.dataset.officeWizardField,
      value: field.type === "checkbox" ? Boolean(field.checked) : field.value,
    });
  };

  const onClick = (event) => {
    const control = event.target.closest?.("[data-office-wizard-action]");
    if (!control || control.disabled) return;
    const action = control.dataset.officeWizardAction;
    if (action === "close") return onClose();
    if (action === "start") {
      const state = getState();
      if (!canStartProvisioning(state)) {
        return dispatch({ type: "set_status", status: "error", errorCode: "byol_not_accepted" });
      }
      if (!canContinueFromPreflight(state)) {
        return dispatch({ type: "set_status", status: "error", errorCode: "preflight_blocked" });
      }
      return onStart();
    }
    if (action === "set-step") {
      return dispatch({ type: "set_step", step: control.dataset.officeWizardStep });
    }
    if (action === "launch-app") {
      const state = getState();
      return onLaunchApp({
        name: String(state.profileName || "office"),
        appId: String(control.dataset.officeApp || ""),
        files: [],
        guiProgress: true,
      });
    }
    if (action === "next" || action === "back") return dispatch({ type: action });
    return undefined;
  };

  const onKeydown = (event) => {
    if (event.key === "Escape") onClose();
  };

  root.addEventListener("input", onInput);
  root.addEventListener("change", onInput);
  root.addEventListener("click", onClick);
  root.addEventListener("keydown", onKeydown);

  return () => {
    root.removeEventListener("input", onInput);
    root.removeEventListener("change", onInput);
    root.removeEventListener("click", onClick);
    root.removeEventListener("keydown", onKeydown);
  };
}

function stateFromControls(root) {
  const byol = root.querySelector?.('[data-office-wizard-field="byolAccepted"]');
  const telemetry = root.querySelector?.('[data-office-wizard-field="telemetryOptIn"]');
  const warning = root.querySelector?.('[data-office-wizard-field="warningOverride"]');
  const adoption = root.querySelector?.('[data-office-wizard-field="adoptionConfirmed"]');
  return initialOfficeWizardState({
    byolAccepted: Boolean(byol?.checked),
    telemetryOptIn: Boolean(telemetry?.checked),
    warningOverride: Boolean(warning?.checked),
    adoptionConfirmed: Boolean(adoption?.checked),
  });
}

function renderStepItem(step, index, currentIndex, { t, escapeHtml, escapeAttr }) {
  const state = index < currentIndex ? "done" : index === currentIndex ? "running" : "pending";
  return `
    <button type="button" class="office-wizard-step" data-status="${escapeAttr(state)}"
            data-office-wizard-action="set-step" data-office-wizard-step="${escapeAttr(step)}"
            aria-current="${index === currentIndex ? "step" : "false"}">
      <span>${escapeHtml(String(index + 1))}</span>
      <strong>${escapeHtml(t(`officeWizard.step.${step}`))}</strong>
    </button>`;
}

function renderCurrentStep(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  if (state.step === "profile") {
    return `
      <div class="office-wizard-copy">
        <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.profile.title"))}</h3>
        <p>${escapeHtml(t("officeWizard.profile.desc"))}</p>
      </div>
      <div class="office-wizard-fields">
        <label class="field" for="office-profile-name">
          <span class="field-label">${escapeHtml(t("officeWizard.field.profileName"))}</span>
          <input id="office-profile-name" class="field-input" data-office-wizard-field="profileName"
                 value="${escapeAttr(state.profileName)}" pattern="[a-z0-9][a-z0-9_-]*" autocomplete="off" />
        </label>
        <label class="field" for="office-language">
          <span class="field-label">${escapeHtml(t("officeWizard.field.language"))}</span>
          <select id="office-language" class="field-select" data-office-wizard-field="language">
            ${option("pt-br", state.language, t("officeWizard.language.ptBr"), deps)}
            ${option("en-us", state.language, t("officeWizard.language.enUs"), deps)}
          </select>
        </label>
      </div>`;
  }
  if (state.step === "license") {
    return renderByolStep(state, deps);
  }
  if (state.step === "preflight") {
    return renderPreflightStep(state, deps);
  }
  if (state.step === "provisioning") {
    return renderProvisioningStep(state, deps);
  }
  return `
    <div class="office-wizard-copy">
      <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.intro.title"))}</h3>
      <p>${escapeHtml(t("officeWizard.intro.desc"))}</p>
    </div>
    <div class="office-wizard-summary">
      <div><span>${escapeHtml(t("officeWizard.summary.profile"))}</span><strong>${escapeHtml(state.profileName)}</strong></div>
      <div><span>${escapeHtml(t("officeWizard.summary.office"))}</span><strong>${escapeHtml(state.productId)}</strong></div>
    </div>`;
}

export function renderProvisioningStep(state, deps) {
  const { t, escapeHtml } = deps;
  const current = initialOfficeWizardState(state);
  if (isOfficeReadyForLaunch(current)) return renderReadyStep(current, deps);
  const progressMessage = current.lastProgress?.message || "";
  const failure = current.lastError
    ? `<div class="office-wizard-callout" data-tone="danger" role="alert">${escapeHtml(current.lastError.message || current.lastError.code || t("officeWizard.provisioning.failed"))}</div>`
    : "";
  return `
    <div class="office-wizard-copy">
      <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.provisioning.title"))}</h3>
      <p>${escapeHtml(t("officeWizard.provisioning.desc"))}</p>
    </div>
    ${renderProvisioningIntro(deps)}
    ${progressMessage ? `<div class="office-wizard-callout" data-tone="info" aria-live="polite">${escapeHtml(progressMessage)}</div>` : ""}
    ${failure}
    ${renderProvisioningTimeline(current, deps)}`;
}

export function renderReadyStep(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const current = initialOfficeWizardState(state);
  const firstLaunchStatus = current.phaseStatuses.first_launch || "pending";
  return `
    <div class="office-wizard-copy">
      <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.ready.title"))}</h3>
      <p>${escapeHtml(t("officeWizard.ready.desc", { profile: current.profileName }))}</p>
    </div>
    ${renderActivationGuidance(current, deps)}
    <div class="office-wizard-launch-grid" aria-label="${escapeAttr(t("officeWizard.launch.actionsLabel"))}">
      ${OFFICE_LAUNCH_APPS.map(([appId, labelKey]) => `
        <button type="button" class="btn btn-primary" data-office-wizard-action="launch-app"
                data-office-app="${escapeAttr(appId)}">
          ${escapeHtml(t(labelKey))}
        </button>`).join("")}
    </div>
    <div class="office-wizard-callout" data-tone="info">
      ${escapeHtml(t(`officeWizard.ready.firstLaunch.${firstLaunchStatus}`))}
    </div>
    ${renderProvisioningTimeline(current, deps)}`;
}

export function renderActivationGuidance(state, { t, escapeHtml, escapeAttr }) {
  const current = initialOfficeWizardState(state);
  return `
    <section class="office-wizard-activation" aria-label="${escapeAttr(t("officeWizard.activation.title"))}">
      <strong>${escapeHtml(t("officeWizard.activation.title"))}</strong>
      <p>${escapeHtml(t("officeWizard.activation.remoteApp", { profile: current.profileName }))}</p>
      <p>${escapeHtml(t("officeWizard.activation.userResponsibility"))}</p>
      <p>${escapeHtml(t("officeWizard.activation.noAutomation"))}</p>
    </section>`;
}

export function renderProvisioningIntro({ t, escapeHtml }) {
  return `
    <div class="office-wizard-provisioning-intro" data-testid="office-provisioning-intro">
      <strong>${escapeHtml(t("officeWizard.provisioning.durationTitle"))}</strong>
      <p>${escapeHtml(t("officeWizard.provisioning.duration"))}</p>
      <p>${escapeHtml(t("officeWizard.provisioning.largeDownload"))}</p>
    </div>`;
}

export function renderProvisioningTimeline(state, deps) {
  return renderPhaseList(initialOfficeWizardState(state), deps, { all: true });
}

export function renderPreflightStep(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const current = initialOfficeWizardState(state);
  const checks = preflightChecks(current);
  const blockers = checks.filter(check => preflightStatus(check) === "blocker");
  const warnings = checks.filter(check => preflightStatus(check) === "warning");
  const loading = current.status === "loading"
    ? `<div class="office-wizard-callout" data-tone="info" aria-live="polite">${escapeHtml(t("officeWizard.preflight.loading"))}</div>`
    : "";
  const error = current.status === "error" && current.errorCode === "preflight_blocked"
    ? `<div class="office-wizard-callout" data-tone="danger" role="alert">${escapeHtml(t("officeWizard.preflight.blocked"))}</div>`
    : "";
  const checkList = checks.length
    ? `<div class="office-wizard-preflight-list" aria-label="${escapeAttr(t("officeWizard.preflight.checksLabel"))}">
        ${checks.map(check => renderPreflightCheck(check, deps)).join("")}
      </div>`
    : `<div class="office-wizard-callout" data-tone="warn">${escapeHtml(t("officeWizard.preflight.duration"))}</div>`;
  const override = warnings.length && !blockers.length
    ? renderWarningOverride(current, warnings, deps)
    : "";

  return `
    <div class="office-wizard-copy">
      <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.preflight.title"))}</h3>
      <p>${escapeHtml(t("officeWizard.preflight.desc"))}</p>
    </div>
    ${loading}
    ${error}
    ${checkList}
    ${override}
    ${renderAdoptionReview(current, deps)}`;
}

export function renderAdoptionReview(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const current = initialOfficeWizardState(state);
  const candidates = adoptionCandidates(current);
  if (!candidates.length) return "";
  const managed = managedScopeItems(current, true, deps);
  const preserved = managedScopeItems(current, false, deps);
  return `
    <section class="office-wizard-adoption" aria-label="${escapeAttr(t("officeWizard.adoption.title"))}">
      <div class="office-wizard-copy">
        <h3>${escapeHtml(t("officeWizard.adoption.title"))}</h3>
        <p>${escapeHtml(t("officeWizard.adoption.desc"))}</p>
      </div>
      <div class="office-wizard-adoption-findings">
        ${candidates.map(candidate => renderAdoptionFinding(candidate, deps)).join("")}
      </div>
      <div class="office-wizard-scope-grid">
        <div>
          <strong>${escapeHtml(t("officeWizard.adoption.managed"))}</strong>
          <ul>${managed}</ul>
        </div>
        <div>
          <strong>${escapeHtml(t("officeWizard.adoption.preserved"))}</strong>
          <ul>${preserved}</ul>
        </div>
      </div>
      <label class="office-wizard-check">
        <input type="checkbox" data-office-wizard-field="adoptionConfirmed" ${current.adoptionConfirmed ? "checked" : ""} />
        <span>${escapeHtml(t("officeWizard.adoption.confirm"))}</span>
      </label>
    </section>`;
}

export function renderByolStep(state, deps) {
  const { t, escapeHtml } = deps;
  const error = state.errorCode === "byol_not_accepted"
    ? `<div class="office-wizard-callout" data-tone="danger" role="alert">${escapeHtml(t("officeWizard.byol.error"))}</div>`
    : "";
  const loading = state.status === "loading"
    ? `<div class="office-wizard-callout" data-tone="info" aria-live="polite">${escapeHtml(t("officeWizard.byol.loading"))}</div>`
    : "";
  return `
    <div class="office-wizard-copy">
      <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.license.title"))}</h3>
      <p id="office-wizard-byol-desc">${escapeHtml(t("officeWizard.license.desc"))}</p>
    </div>
    <ul class="office-wizard-disclaimer" aria-describedby="office-wizard-byol-desc">
      ${BYOL_DISCLAIMER_KEYS.map(key => `<li>${escapeHtml(t(key))}</li>`).join("")}
    </ul>
    ${renderLicenseScopeStep(state, deps)}
    ${renderThirdPartyAttributions(deps)}
    ${renderTelemetryPreference(state, deps)}
    ${error}
    ${loading}
    <label class="office-wizard-check">
      <input type="checkbox" data-office-wizard-field="byolAccepted"
             aria-describedby="office-wizard-byol-desc" ${state.byolAccepted ? "checked" : ""} />
      <span>${escapeHtml(t("officeWizard.license.accept"))}</span>
    </label>`;
}

export function renderTelemetryPreference(state, { t, escapeHtml }) {
  const current = initialOfficeWizardState(state);
  return `
    <label class="office-wizard-check">
      <input type="checkbox" data-office-wizard-field="telemetryOptIn"
             ${current.telemetryOptIn ? "checked" : ""} />
      <span>
        <strong>${escapeHtml(t("officeWizard.telemetry.title"))}</strong>
        <small>${escapeHtml(t("officeWizard.telemetry.desc"))}</small>
      </span>
    </label>`;
}

export function renderLicenseScopeStep(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const href = safeGuideHref(deps.licenseAdrHref || "adrs/adr-agpl-winapps-runtime-boundary.md");
  return `
    <section class="office-wizard-warning-override" aria-label="${escapeAttr(t("officeWizard.licenseInfo.title"))}">
      <strong>${escapeHtml(t("officeWizard.licenseInfo.title"))}</strong>
      <ul class="office-wizard-disclaimer">
        ${LICENSE_SCOPE_KEYS.map(key => `<li>${escapeHtml(t(key))}</li>`).join("")}
      </ul>
      <a class="office-wizard-action-hint" href="${escapeAttr(href)}" rel="noreferrer">
        ${escapeHtml(t("officeWizard.licenseInfo.adrLink"))}
      </a>
    </section>`;
}

export function renderThirdPartyAttributions({ t, escapeHtml, escapeAttr, thirdPartyHref }) {
  const href = safeGuideHref(thirdPartyHref || "THIRD-PARTY.md");
  return `
    <section class="office-wizard-warning-override" aria-label="${escapeAttr(t("officeWizard.thirdParty.title"))}">
      <strong>${escapeHtml(t("officeWizard.thirdParty.title"))}</strong>
      <p>${escapeHtml(t("officeWizard.thirdParty.desc"))}</p>
      <a class="office-wizard-action-hint" href="${escapeAttr(href)}" rel="noreferrer">
        ${escapeHtml(t("officeWizard.thirdParty.link"))}
      </a>
    </section>`;
}

function safeGuideHref(value) {
  const raw = String(value || "").trim();
  if (!raw) return "#";
  if (/^https:\/\/[^\s"'<>]+$/i.test(raw)) return raw;
  if (/^(?:\.{0,2}\/)?[a-z0-9_./-]+$/i.test(raw)) return raw;
  return "#";
}

function byolNotAcceptedState(state) {
  return {
    ...state,
    status: "error",
    errorCode: "byol_not_accepted",
    phaseStatuses: {
      ...state.phaseStatuses,
      byol_acceptance: "failed",
    },
  };
}

function renderPhaseList(state, deps, options = {}) {
  const { t, escapeHtml, escapeAttr } = deps;
  const phases = options.all ? OFFICE_WIZARD_PHASES : OFFICE_WIZARD_PHASES.slice(0, 6);
  return `
    <ol class="office-wizard-phases" aria-label="${escapeAttr(t("officeWizard.phasesLabel"))}">
      ${phases.map(phase => {
        const status = state.phaseStatuses[phase] || "pending";
        return `
          <li data-status="${escapeAttr(status)}">
            <span class="office-wizard-phase-dot" aria-hidden="true"></span>
            <span>${escapeHtml(officeWizardPhaseLabel(phase, { t }))}</span>
            <strong>${escapeHtml(t(`officeWizard.status.${status}`))}</strong>
          </li>`;
      }).join("")}
    </ol>`;
}

function progressStatusToPhaseStatus(status) {
  const normalized = String(status || "").toLowerCase();
  if (["success", "complete", "completed", "done"].includes(normalized)) return "done";
  if (["error", "failed", "failure"].includes(normalized)) return "failed";
  if (["skipped", "skip"].includes(normalized)) return "skipped";
  if (normalized === "running") return "running";
  return "pending";
}

function normalizePhaseStatuses(phases) {
  if (!phases || typeof phases !== "object") return {};
  return Object.fromEntries(
    OFFICE_WIZARD_PHASES
      .filter(phase => phases[phase])
      .map(phase => [phase, normalizePhaseStatusValue(phases[phase])]),
  );
}

function normalizePhaseStatusValue(value) {
  if (typeof value === "string") return progressStatusToPhaseStatus(value);
  return progressStatusToPhaseStatus(value?.status || "pending");
}

function hasStartedProvisioning(phases, status, lastError) {
  if (lastError) return true;
  if (["running", "blocked", "ready", "failed", "adopted"].includes(status)) return true;
  return Object.values(phases).some(value => value !== "pending");
}

function renderPreflightCheck(check, { t, escapeHtml, escapeAttr }) {
  const status = preflightStatus(check);
  const actionHint = check.action_hint || check.actionHint || "";
  const role = status === "blocker" ? " role=\"alert\"" : "";
  return `
    <article class="office-wizard-preflight-check" data-status="${escapeAttr(status)}"${role}>
      <div>
        <strong>${escapeHtml(check.requirement || check.id || t("officeWizard.preflight.unknownRequirement"))}</strong>
        <span>${escapeHtml(t(`officeWizard.preflight.status.${status}`))}</span>
      </div>
      <p>${escapeHtml(check.impact || t("officeWizard.preflight.noImpact"))}</p>
      ${actionHint ? `<p class="office-wizard-action-hint">${escapeHtml(actionHint)}</p>` : ""}
    </article>`;
}

function renderWarningOverride(state, warnings, { t, escapeHtml }) {
  return `
    <div class="office-wizard-warning-override" role="group" aria-label="${escapeHtml(t("officeWizard.warningOverride.title"))}">
      <strong>${escapeHtml(t("officeWizard.warningOverride.title"))}</strong>
      <p>${escapeHtml(t("officeWizard.warningOverride.desc", { count: warnings.length }))}</p>
      <label class="office-wizard-check">
        <input type="checkbox" data-office-wizard-field="warningOverride" ${state.warningOverride ? "checked" : ""} />
        <span>${escapeHtml(t("officeWizard.warningOverride.confirm"))}</span>
      </label>
    </div>`;
}

function renderAdoptionFinding(candidate, { t, escapeHtml, escapeAttr }) {
  const status = String(candidate.status || "partial");
  return `
    <article class="office-wizard-adoption-finding" data-status="${escapeAttr(status)}">
      <div>
        <strong>${escapeHtml(candidate.kind || candidate.id || t("officeWizard.adoption.unknown"))}</strong>
        <span>${escapeHtml(t(`officeWizard.adoption.status.${status}`))}</span>
      </div>
      <p>${escapeHtml(candidate.evidence || "")}</p>
      <small>${escapeHtml(candidate.managedByDefault ? t("officeWizard.adoption.defaultManaged") : t("officeWizard.adoption.defaultPreserved"))}</small>
    </article>`;
}

function managedScopeItems(state, managed, { t, escapeHtml }) {
  const items = MANAGED_SCOPE_KEYS
    .filter(([key]) => Boolean(state.managedScope?.[key]) === managed)
    .map(([, labelKey]) => `<li>${escapeHtml(t(labelKey))}</li>`);
  if (state.managedScope?.preserveExistingWinAppsClone === !managed) {
    items.push(`<li>${escapeHtml(t("officeWizard.adoption.scope.winappsClone"))}</li>`);
  }
  return items.length ? items.join("") : `<li>${escapeHtml(t("officeWizard.adoption.none"))}</li>`;
}

function preflightChecks(state) {
  return Array.isArray(state.preflightChecks) ? state.preflightChecks : [];
}

function adoptionCandidates(state) {
  return Array.isArray(state.adoptionCandidates) ? state.adoptionCandidates : [];
}

function preflightStatus(check) {
  const status = String(check?.status || "ok").toLowerCase();
  if (status === "warn") return "warning";
  if (["ok", "warning", "blocker"].includes(status)) return status;
  return "blocker";
}

function preflightHasResult(state) {
  return preflightChecks(state).length > 0 || adoptionCandidates(state).length > 0;
}

function preflightBlockReason(state, { t }) {
  const checks = preflightChecks(state);
  if (checks.some(check => preflightStatus(check) === "blocker")) {
    return t("officeWizard.preflight.reason.blocker");
  }
  if (checks.some(check => preflightStatus(check) === "warning") && !state.warningOverride) {
    return t("officeWizard.preflight.reason.warning");
  }
  if (adoptionCandidates(state).length > 0 && !state.adoptionConfirmed) {
    return t("officeWizard.preflight.reason.adoption");
  }
  return "";
}

function option(value, selected, label, { escapeHtml, escapeAttr }) {
  return `<option value="${escapeAttr(value)}" ${selected === value ? "selected" : ""}>${escapeHtml(label)}</option>`;
}
