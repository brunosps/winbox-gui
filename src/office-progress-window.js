import {
  applyOfficeProgressEvent,
  initialOfficeWizardState,
  renderProvisioningTimeline,
} from "./office-wizard.js";

export function initialOfficeProgressWindowState(overrides = {}) {
  return {
    profileName: "office",
    appId: "",
    closed: false,
    ...overrides,
    wizardState: initialOfficeWizardState({
      profileName: overrides.profileName || "office",
      step: "provisioning",
      ...(overrides.wizardState || {}),
    }),
  };
}

export function officeProgressWindowReducer(state, action = {}) {
  const current = initialOfficeProgressWindowState(state);
  if (action.type === "operation_progress") {
    return {
      ...current,
      wizardState: applyOfficeProgressEvent(current.wizardState, action.event || {}),
    };
  }
  if (action.type === "close") return { ...current, closed: true };
  return current;
}

export function renderOfficeProgressWindow(state, deps) {
  const { t, escapeHtml, escapeAttr } = deps;
  const current = initialOfficeProgressWindowState(state);
  const progress = current.wizardState.lastProgress || {};
  const message = progress.message || t("officeProgress.waiting");
  return `
    <section class="office-progress-window" aria-label="${escapeAttr(t("officeProgress.title"))}">
      <div class="office-progress-window-header">
        <div>
          <span class="eyebrow">${escapeHtml(t("officeProgress.eyebrow"))}</span>
          <h2>${escapeHtml(t("officeProgress.title"))}</h2>
          <p>${escapeHtml(t("officeProgress.subtitle", {
            profile: current.profileName,
            app: current.appId || t("officeProgress.officeApp"),
          }))}</p>
        </div>
      </div>
      <div class="office-progress-window-status" aria-live="polite">
        <span class="card-progress-spinner" aria-hidden="true"></span>
        <strong>${escapeHtml(message)}</strong>
      </div>
      ${renderProvisioningTimeline(current.wizardState, deps)}
      <p class="office-progress-window-hint">${escapeHtml(t("officeProgress.closeHint"))}</p>
    </section>`;
}

export function bindOfficeProgressWindow(root, deps = {}) {
  const dispatch = deps.dispatch || (() => {});
  const onEvent = (event) => dispatch({ type: "operation_progress", event: event.payload || event });
  const unlisten = deps.listen ? deps.listen("operation-progress", onEvent) : null;
  return () => {
    if (typeof unlisten === "function") unlisten();
  };
}
