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

export const OFFICE_WIZARD_STEPS = ["intro", "profile", "license", "preflight"];

export const OFFICE_WIZARD_PHASES = Object.keys(DEFAULT_PHASE_STATUSES);

export function initialOfficeWizardState(overrides = {}) {
  return {
    step: "intro",
    profileName: "office",
    productId: "O365ProPlusRetail",
    language: "pt-br",
    byolAccepted: false,
    warningOverride: false,
    status: "default",
    errorCode: "",
    phaseStatuses: { ...DEFAULT_PHASE_STATUSES },
    ...overrides,
    phaseStatuses: {
      ...DEFAULT_PHASE_STATUSES,
      ...(overrides.phaseStatuses || {}),
    },
  };
}

export function officeWizardReducer(state, action = {}) {
  const current = initialOfficeWizardState(state);
  if (action.type === "field") {
    return { ...current, [action.field]: action.value };
  }
  if (action.type === "next") {
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
  return current;
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
    return {
      action: "start",
      label: t("officeWizard.cta.startPreflight"),
      disabled: false,
    };
  }
  return { action: "next", label: t("officeWizard.cta.continue"), disabled: false };
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
    if (action === "start") return onStart();
    if (action === "set-step") {
      return dispatch({ type: "set_step", step: control.dataset.officeWizardStep });
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
    return `
      <div class="office-wizard-copy">
        <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.license.title"))}</h3>
        <p>${escapeHtml(t("officeWizard.license.desc"))}</p>
      </div>
      <label class="office-wizard-check">
        <input type="checkbox" data-office-wizard-field="byolAccepted" ${state.byolAccepted ? "checked" : ""} />
        <span>${escapeHtml(t("officeWizard.license.accept"))}</span>
      </label>`;
  }
  if (state.step === "preflight") {
    return `
      <div class="office-wizard-copy">
        <h3 id="office-wizard-current-title">${escapeHtml(t("officeWizard.preflight.title"))}</h3>
        <p>${escapeHtml(t("officeWizard.preflight.desc"))}</p>
      </div>
      <div class="office-wizard-callout" data-tone="warn">${escapeHtml(t("officeWizard.preflight.duration"))}</div>`;
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

function renderPhaseList(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  return `
    <ol class="office-wizard-phases" aria-label="${escapeAttr(t("officeWizard.phasesLabel"))}">
      ${OFFICE_WIZARD_PHASES.slice(0, 6).map(phase => {
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

function option(value, selected, label, { escapeHtml, escapeAttr }) {
  return `<option value="${escapeAttr(value)}" ${selected === value ? "selected" : ""}>${escapeHtml(label)}</option>`;
}
