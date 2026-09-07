const { invoke } = window.__TAURI__.core;
const tauriEvent = window.__TAURI__.event;

import { t, applyAll as applyI18n, getLocale, setLocale } from "./i18n.js";
import { cssToken, escapeAttr, escapeHtml } from "./dom-utils.js";
import {
  modeMeta,
  primaryActionMeta as primaryActionMetaImpl,
  profileState,
} from "./profile-display.js";
import {
  applyOfficeProgressEvent,
  bindOfficeWizard,
  initialOfficeWizardState,
  officeWizardReducer,
  renderOfficeWizard,
  stateFromProvisioningResponse,
} from "./office-wizard.js";
import {
  initialOfficeProgressWindowState,
  officeProgressWindowReducer,
  renderOfficeProgressWindow,
} from "./office-progress-window.js";
import "./locales/en-US.js";
import "./locales/pt-BR.js";

const $  = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const main = $("#main");
const versionEl = $("#version");
const toast = $("#toast");
const toastMsg = $("#toast-msg");

// ─── SVG icon set ─────────────────────────────────────────────────────
const ICO = {
  play:    '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7L8 5z"/></svg>',
  pause:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="9" y1="5" x2="9" y2="19"/><line x1="15" y1="5" x2="15" y2="19"/></svg>',
  stop:    '<svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>',
  more:    '<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/></svg>',
  gear:    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
  logs:    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="21" y1="10" x2="3" y2="10"/><line x1="21" y1="6" x2="3" y2="6"/><line x1="21" y1="14" x2="3" y2="14"/><line x1="17" y1="18" x2="3" y2="18"/></svg>',
  snapshot:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h3l2-2h6l2 2h3v12H4z"/><circle cx="12" cy="13" r="3"/></svg>',
  download:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="M7 10l5 5 5-5"/><path d="M12 15V3"/></svg>',
  star:    '<svg viewBox="0 0 24 24" fill="currentColor"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>',
  trash:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>',
  bolt:    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
  close:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>',
  check:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>',
  alert:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v4"/><path d="M12 16h.01"/></svg>',
  reapply: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7"/><polyline points="21 4 21 10 15 10"/></svg>',
};

// ─── Toast ────────────────────────────────────────────────────────────
let toastTimer;
function showToast(msg, kind = "info") {
  toastMsg.textContent = msg;
  toast.className = "toast visible " + kind;
  const ico = {
    success: ICO.check,
    error:   ICO.alert,
  }[kind] || ICO.alert;
  toast.querySelector(".toast-ico").outerHTML = `<span class="toast-ico">${ico}</span>`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("visible"), 4000);
}

function formatErrorForToast(err) {
  const raw = String(err?.message ?? err ?? "");
  return raw.replace(/\nPróxima ação:/g, `\n${t("toast.nextAction")}:`);
}

function showErrorToast(err) {
  showToast(t("toast.genericError", { msg: formatErrorForToast(err) }), "error");
}

// Render a structured LaunchError ({code, ...fields}) emitted by the
// Tauri launch_profile command. Falls back to showErrorToast for
// string errors or unknown shapes.
function renderLaunchError(err) {
  if (!err || typeof err !== "object" || typeof err.code !== "string") {
    return showErrorToast(err);
  }
  const key = `launch.error.${err.code}`;
  const message = t(key, err);
  // t() returns the key verbatim when missing — fall back to generic in that case.
  if (message === key) {
    return showErrorToast(err.message || err.code);
  }
  showToast(message, "error");
}

// Background errors raised by launch/install/update/restart land here.
if (tauriEvent && tauriEvent.listen) {
  tauriEvent.listen("profile-error", (ev) => {
    const p = ev.payload || {};
    showToast(
      t("toast.profileError", {
        profile: p.profile || "?",
        op: p.op || "?",
        msg: formatErrorForToast(p.message || "?"),
      }),
      "error",
    );
  });
}

// ─── Status label ─────────────────────────────────────────────────────
const statusLabel = (s) => {
  const key = `status.${s}`;
  const tx = t(key);
  return tx === key ? s : tx;
};

function statusTone(status) {
  if (status === "running") return "online";
  if (status === "paused") return "paused";
  return "offline";
}

let selectedProfileName = null;
let profileFilter = "all";
let profileQuery = "";
let latestHealth = null;
let healthLoadedAt = 0;
const HEALTH_TTL_MS = 30000;
const operationEvents = [];
let officeWizardOpen = false;
let officeWizardState = initialOfficeWizardState();
let unbindOfficeWizard = null;
const officeProgressWindowArgs = detectOfficeProgressWindowArgs();
let officeProgressWindowState = initialOfficeProgressWindowState({
  profileName: officeProgressWindowArgs?.profileName || "office",
  appId: officeProgressWindowArgs?.appId || "",
});

// Active launch operations indexed by profile name. Updated as
// operation-progress events arrive so the card can show the current
// step (e.g. "Aguardando Windows iniciar...") instead of a silent
// disabled button during the multi-minute boot wait.
const activeOps = new Map();

if (tauriEvent && tauriEvent.listen) {
  tauriEvent.listen("operation-progress", (ev) => {
    const payload = ev.payload || {};
    addOperationEvent(payload);
    if (officeProgressWindowArgs) {
      officeProgressWindowState = officeProgressWindowReducer(officeProgressWindowState, {
        type: "operation_progress",
        event: payload,
      });
      renderOfficeProgressWindowScreen();
    }
    if (officeWizardOpen) {
      officeWizardState = applyOfficeProgressEvent(officeWizardState, payload);
      renderOfficeWizardScreen();
    }
    renderOperationPanel();
    const status = String(payload.status || "");
    const profile = String(payload.profile || "");
    if (!profile) return;
    if (status === "running") {
      activeOps.set(profile, {
        op: String(payload.op || ""),
        step: String(payload.step || ""),
        message: String(payload.message || ""),
      });
    } else {
      // complete | success | failed | error → clear inline status.
      activeOps.delete(profile);
    }
    renderProfileCardInline(profile);
  });
}

function detectOfficeProgressWindowArgs() {
  const injected = window.__WINBOX_OFFICE_PROGRESS__;
  if (injected && typeof injected === "object") {
    return {
      profileName: String(injected.profileName || injected.profile || "office"),
      appId: String(injected.appId || injected.app || ""),
    };
  }
  const params = new URLSearchParams(window.location.search);
  if (params.get("window") !== "office-progress" && window.location.hash !== "#office-progress") {
    return null;
  }
  return {
    profileName: params.get("profile") || "office",
    appId: params.get("app") || "",
  };
}

// Update only the running-state region of a single card without
// re-rendering the whole list (which would steal focus from any open
// menu / input).
function renderProfileCardInline(profileName) {
  const card = document.querySelector(`.profile-detail[data-profile="${CSS.escape(profileName)}"]`);
  if (!card) return;
  const slot = card.querySelector(".card-progress");
  if (!slot) return;
  const op = activeOps.get(profileName);
  if (!op) {
    slot.innerHTML = "";
    slot.removeAttribute("data-running");
    return;
  }
  slot.setAttribute("data-running", "true");
  slot.innerHTML = `
    <span class="card-progress-spinner" aria-hidden="true"></span>
    <span class="card-progress-msg">${escapeHtml(op.message || op.step || "Processando...")}</span>
  `;
}

function profileName(p) {
  return String(p.name || "");
}

function bundlesForProfile(p) {
  if (!p.bundles || p.bundles === "essentials") return ["essentials"];
  const list = String(p.bundles).split(",").map(b => b.trim()).filter(Boolean);
  return list.length ? list : ["essentials"];
}

function osMeta(p) {
  const family = p.image_family || "windows";
  if (family === "windows") return { label: "Windows", className: "is-windows" };
  if (family === "linux_iso") return { label: "ISO", className: "is-iso" };
  return { label: p.boot || "Linux", className: "is-linux" };
}

function primaryActionMeta(p) {
  return primaryActionMetaImpl(p, { t, icons: ICO });
}

function matchesProfile(p) {
  const state = profileState(p);
  if (profileFilter !== "all" && state !== profileFilter) return false;
  const q = profileQuery.trim().toLowerCase();
  if (!q) return true;
  const haystack = [
    profileName(p),
    state,
    p.image_family,
    p.boot,
    p.connect_mode,
    p.ram,
    p.web_port,
    p.rdp_port,
    p.bundles,
  ].join(" ").toLowerCase();
  return haystack.includes(q);
}

function profileSort(a, b) {
  if (a.is_default !== b.is_default) return a.is_default ? -1 : 1;
  const rank = { running: 0, paused: 1, created: 2, exited: 3, absent: 4 };
  const ar = rank[profileState(a)] ?? 5;
  const br = rank[profileState(b)] ?? 5;
  if (ar !== br) return ar - br;
  return profileName(a).localeCompare(profileName(b));
}

function chooseSelectedProfile(profiles) {
  if (!profiles.length) {
    selectedProfileName = null;
    return null;
  }
  const selected = profiles.find(p => profileName(p) === selectedProfileName) || profiles[0];
  selectedProfileName = profileName(selected);
  return selected;
}

function summaryStats(profiles) {
  const running = profiles.filter(p => profileState(p) === "running").length;
  const paused = profiles.filter(p => profileState(p) === "paused").length;
  const stopped = profiles.length - running - paused;
  return [
    { label: t("dashboard.summary.total"), value: profiles.length, tone: "neutral" },
    { label: t("dashboard.summary.running"), value: running, tone: "success" },
    { label: t("dashboard.summary.paused"), value: paused, tone: "warning" },
    { label: t("dashboard.summary.stopped"), value: stopped, tone: "muted" },
  ];
}

function opsMetricTemplate(item) {
  return `
    <div class="ops-metric" data-tone="${escapeAttr(item.tone)}">
      <span>${escapeHtml(item.label)}</span>
      <strong>${escapeHtml(item.value)}</strong>
    </div>`;
}

async function loadHostHealth({ force = false } = {}) {
  const freshEnough = latestHealth && Date.now() - healthLoadedAt < HEALTH_TTL_MS;
  if (!force && freshEnough) return latestHealth;
  try {
    latestHealth = await invoke("host_health");
    healthLoadedAt = Date.now();
    return latestHealth;
  } catch {
    latestHealth = null;
    healthLoadedAt = 0;
    return null;
  }
}

function healthLabel(status) {
  const key = `health.status.${status || "unknown"}`;
  const tx = t(key);
  return tx === key ? t("health.status.unknown") : tx;
}

function formattedTime(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return date.toLocaleTimeString(getLocale(), {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function healthCompactTemplate(report) {
  if (!report) {
    return `
      <section class="ops-health" aria-label="${t("health.title")}">
        <div class="ops-block-head">
          <span>${t("health.eyebrow")}</span>
          <button type="button" class="btn btn-ghost btn-sm" data-refresh-health>${t("health.refresh")}</button>
        </div>
        <div class="health-unavailable">${t("health.unavailable")}</div>
      </section>`;
  }

  const checks = Array.isArray(report.checks) ? report.checks : [];
  const counts = checks.reduce((acc, check) => {
    const status = check.status || "unknown";
    acc[status] = (acc[status] || 0) + 1;
    return acc;
  }, { ok: 0, warn: 0, error: 0 });
  const issueChecks = checks
    .filter(check => check.status === "warn" || check.status === "error")
    .sort((a, b) => (a.status === "error" ? 0 : 1) - (b.status === "error" ? 0 : 1));
  const okChecks = checks.filter(check => check.status === "ok");
  const primaryIssue = issueChecks[0] || null;
  const issueTitle = primaryIssue
    ? `${primaryIssue.label || primaryIssue.id || "check"} · ${healthLabel(primaryIssue.status)}`
    : t("health.allClear.title");
  const issueMessage = primaryIssue
    ? primaryIssue.message || primaryIssue.action_hint || ""
    : t("health.allClear.desc");

  return `
    <section class="ops-health" aria-label="${t("health.title")}" data-status="${escapeAttr(report.overall_status || "unknown")}">
      <div class="ops-block-head">
        <span>${t("health.eyebrow")}</span>
        <div class="ops-block-actions">
          <span class="health-summary" data-status="${escapeAttr(report.overall_status || "unknown")}">${healthLabel(report.overall_status)}</span>
          <button type="button" class="btn btn-ghost btn-sm" data-refresh-health>${t("health.refresh")}</button>
        </div>
      </div>
      <div class="ops-health-focus" data-status="${escapeAttr(primaryIssue?.status || "ok")}">
        <strong>${escapeHtml(issueTitle)}</strong>
        <span>${escapeHtml(issueMessage)}</span>
      </div>
      <div class="health-counts" aria-label="${t("health.metrics")}">
        <span data-status="ok">${escapeHtml(counts.ok || 0)} ${t("health.status.ok")}</span>
        <span data-status="warn" data-active="${counts.warn ? "1" : "0"}">${escapeHtml(counts.warn || 0)} ${t("health.status.warn")}</span>
        <span data-status="error" data-active="${counts.error ? "1" : "0"}">${escapeHtml(counts.error || 0)} ${t("health.status.error")}</span>
      </div>
      ${okChecks.length ? `<div class="health-ok-inline">${okChecks.slice(0, 4).map(check => `<span>${escapeHtml(check.label || check.id || "check")}</span>`).join("")}</div>` : ""}
      <div class="health-footer">${t("health.generated", { time: formattedTime(report.generated_at) })}</div>
    </section>`;
}

function addOperationEvent(event) {
  if (!event || !event.profile || !event.op) return;
  operationEvents.unshift({
    profile: String(event.profile),
    op: String(event.op),
    step: String(event.step || ""),
    status: String(event.status || "running"),
    message: String(event.message || ""),
    at: new Date().toISOString(),
  });
  operationEvents.splice(12);
}

function operationLabel(event) {
  const key = `ops.op.${event.op}`;
  const tx = t(key);
  return tx === key ? event.op : tx;
}

function operationStatusLabel(status) {
  const key = `ops.status.${status || "running"}`;
  const tx = t(key);
  return tx === key ? status : tx;
}

function operationPanelTemplate() {
  const rows = operationEvents.length
    ? operationEvents.slice(0, 2).map(event => `
        <article class="operation-row" data-status="${escapeAttr(event.status)}">
          <div class="operation-row-head">
            <strong>${escapeHtml(operationLabel(event))}</strong>
            <span>${operationStatusLabel(event.status)}</span>
          </div>
          <div class="operation-meta">
            <span>${escapeHtml(event.profile)}</span>
            ${event.step ? `<span>${escapeHtml(event.step)}</span>` : ""}
            <span>${formattedTime(event.at)}</span>
          </div>
          ${event.message ? `<p>${escapeHtml(formatErrorForToast(event.message))}</p>` : ""}
        </article>
      `).join("")
    : `<div class="operation-empty">${t("ops.empty")}</div>`;

  return `
    <section id="operation-panel" class="ops-events" aria-label="${t("ops.title")}">
      <div class="ops-block-head">
        <span>${t("ops.eyebrow")}</span>
        <strong>${t("ops.title")}</strong>
      </div>
      <div class="operation-list">${rows}</div>
    </section>`;
}

function renderOperationPanel() {
  const panel = $("#operation-panel");
  if (!panel) return;
  panel.outerHTML = operationPanelTemplate();
}

function opsBarTemplate(profiles, health) {
  // Header dropped on 2026-05-19 per UX request — the "Fleet console"
  // eyebrow + title + subtitle were repetitive next to the existing
  // app header and added vertical noise without orientation value.
  // Health, operations panel and metrics stay; layout falls back from
  // 4 blocks to 3 cleanly via existing CSS auto-flow.
  return `
    <section class="ops-bar" aria-label="${t("dashboard.overview")}">
      ${healthCompactTemplate(health)}
      ${operationPanelTemplate()}
      <div class="ops-metrics" aria-label="${t("dashboard.overview")}">
        ${summaryStats(profiles).map(opsMetricTemplate).join("")}
      </div>
    </section>`;
}

function statusPillTemplate(state) {
  return `
    <span class="pill pill-status status-pill" data-status="${escapeAttr(state)}" data-tone="${statusTone(state)}">
      <span class="dot"></span>${escapeHtml(statusLabel(state))}
    </span>`;
}

function badgeGroupTemplate(p) {
  const os = osMeta(p);
  const mode = modeMeta(p);
  return `
    <span class="profile-badges card-badges">
      <span class="badge-os ${os.className}">${escapeHtml(os.label)}</span>
      <span class="badge-connect ${mode.className}">${escapeHtml(mode.label)}</span>
    </span>`;
}

function compactActionsTemplate(p) {
  const state = profileState(p);
  const primary = primaryActionMeta(p);
  const secondary = [];
  if (state === "running") {
    secondary.push({ act: "pause", title: t("card.pause.title"), icon: ICO.pause });
    secondary.push({ act: "stop", title: t("card.stop.title"), icon: ICO.stop });
  } else if (state === "paused") {
    secondary.push({ act: "stop", title: t("card.stop.title"), icon: ICO.stop });
  }
  return `
    <div class="row-actions">
      <button class="btn btn-primary btn-compact" data-act="${primary.act}">
        ${primary.icon}<span>${escapeHtml(primary.label)}</span>
      </button>
      ${secondary.map(a => `
        <button class="btn btn-icon" data-act="${a.act}" title="${escapeAttr(a.title)}" aria-label="${escapeAttr(a.title)}">${a.icon}</button>
      `).join("")}
      <button class="btn btn-icon" data-act="menu" aria-label="${t("card.menu")}" title="${t("card.menu")}" aria-haspopup="true" aria-expanded="false">${ICO.more}</button>
    </div>`;
}

function profileRowTemplate(p, selected) {
  const name = profileName(p);
  const state = profileState(p);
  const bundles = bundlesForProfile(p);
  return `
    <article class="profile-row card ${selected ? "is-selected" : ""}" role="row" tabindex="0"
             data-select-profile data-profile="${escapeAttr(name)}" data-state="${escapeAttr(state)}"
             data-is-default="${p.is_default ? "1" : "0"}" aria-selected="${selected ? "true" : "false"}">
      <div class="profile-cell profile-main" role="cell">
        <div class="profile-title-row">
          <strong class="profile-name">${escapeHtml(name)}</strong>
          ${p.is_default ? `<span class="pill pill-default">${t("card.pillDefault")}</span>` : ""}
        </div>
        ${badgeGroupTemplate(p)}
      </div>
      <div class="profile-cell profile-status-cell" role="cell">${statusPillTemplate(state)}</div>
      <div class="profile-cell profile-resource" role="cell">
        <span class="cell-label">${t("card.spec.ram")}</span>
        <strong>${escapeHtml(p.ram || "—")}</strong>
      </div>
      <div class="profile-cell profile-endpoints" role="cell">
        <span>RDP :${escapeHtml(p.rdp_port || "—")}</span>
        <span>Web :${escapeHtml(p.web_port || "—")}</span>
      </div>
      <div class="profile-cell profile-bundles-cell" role="cell">
        <span>${escapeHtml(bundles.slice(0, 2).join(", "))}</span>
        ${bundles.length > 2 ? `<span class="muted">+${bundles.length - 2}</span>` : ""}
      </div>
      <div class="profile-cell profile-actions-cell" role="cell">${compactActionsTemplate(p)}</div>
    </article>`;
}

function detailActionTemplate(p) {
  const state = profileState(p);
  const primary = primaryActionMeta(p);
  const name = profileName(p);
  const maybeDefault = !p.is_default
    ? `<button class="btn btn-ghost" data-act="default">${ICO.star}<span>${t("card.menu.makeDefault")}</span></button>`
    : "";
  const maybeKill = (state === "running" || state === "paused")
    ? `<button class="btn btn-danger" data-act="kill">${ICO.bolt}<span>${t("card.menu.kill")}</span></button>`
    : "";
  return `
    <div class="detail-actions">
      <button class="btn btn-primary detail-primary" data-act="${primary.act}">
        ${primary.icon}<span>${escapeHtml(primary.label)}</span>
      </button>
      <div class="detail-action-grid">
        ${state === "running" ? `<button class="btn btn-ghost" data-act="pause">${ICO.pause}<span>${t("card.pause.title")}</span></button>` : ""}
        ${(state === "running" || state === "paused") ? `<button class="btn btn-ghost" data-act="stop">${ICO.stop}<span>${t("card.stop.title")}</span></button>` : ""}
        <button class="btn btn-ghost" data-act="settings">${ICO.gear}<span>${t("card.menu.settings")}</span></button>
        <button class="btn btn-ghost" data-act="logs">${ICO.logs}<span>${t("card.menu.logs")}</span></button>
        <button class="btn btn-ghost" data-act="snapshots">${ICO.snapshot}<span>${t("card.menu.snapshots")}</span></button>
        <button class="btn btn-ghost" data-act="reapply">${ICO.reapply}<span>${t("card.menu.reapply")}</span></button>
        <button class="btn btn-ghost" data-act="restart">${ICO.reapply}<span>${t("card.menu.restart")}</span></button>
        <button class="btn btn-ghost" data-act="update-image">${ICO.download}<span>${t("card.menu.updateImage")}</span></button>
        ${maybeDefault}
      </div>
      <div class="detail-danger">
        ${maybeKill}
        <button class="btn btn-danger" data-act="remove">${ICO.trash}<span>${t("card.menu.remove")}</span></button>
      </div>
      <span class="sr-only">${escapeHtml(name)}</span>
    </div>`;
}

function profileDetailTemplate(p) {
  if (!p) {
    return `
      <aside class="profile-detail detail-empty" aria-label="${t("dashboard.detail")}">
        <div class="empty-illustration">${ICO.alert}</div>
        <div class="empty-title">${t("dashboard.noMatches")}</div>
        <div class="empty-desc">${t("dashboard.noMatches.desc")}</div>
      </aside>`;
  }
  const name = profileName(p);
  const state = profileState(p);
  const bundles = bundlesForProfile(p);
  const os = osMeta(p);
  const mode = modeMeta(p);
  return `
    <aside class="profile-detail card" aria-label="${t("dashboard.detail")}"
           data-profile="${escapeAttr(name)}" data-state="${escapeAttr(state)}" data-is-default="${p.is_default ? "1" : "0"}">
      <div class="detail-header">
        <div>
          <span class="eyebrow">${p.is_default ? t("card.kicker.default") : t("card.kicker.profile")}</span>
          <h2>${escapeHtml(name)}</h2>
        </div>
        ${statusPillTemplate(state)}
      </div>
      <div class="detail-meta">
        <span class="badge-os ${os.className}">${escapeHtml(os.label)}</span>
        <span class="badge-connect ${mode.className}">${escapeHtml(mode.label)}</span>
      </div>
      <div class="card-progress" aria-live="polite"></div>
      <dl class="detail-grid">
        <div><dt>${t("card.spec.ram")}</dt><dd>${escapeHtml(p.ram || "—")}</dd></div>
        <div><dt>${t("dashboard.detail.web")}</dt><dd>:${escapeHtml(p.web_port || "—")}</dd></div>
        <div><dt>${t("dashboard.detail.rdp")}</dt><dd>:${escapeHtml(p.rdp_port || "—")}</dd></div>
        <div><dt>${t("card.spec.bundles")}</dt><dd>${bundles.length}</dd></div>
      </dl>
      <div class="detail-section">
        <div class="section-title">${t("dashboard.detail.bundles")}</div>
        <div class="chip-list">${bundles.map(b => `<span class="chip">${escapeHtml(b)}</span>`).join("")}</div>
      </div>
      ${detailActionTemplate(p)}
    </aside>`;
}

function dashboardShell(profiles, health) {
  const orderedProfiles = [...profiles].sort(profileSort);
  const visibleProfiles = orderedProfiles.filter(matchesProfile);
  const selected = chooseSelectedProfile(visibleProfiles);
  const rows = visibleProfiles.length
    ? visibleProfiles.map(p => profileRowTemplate(p, profileName(p) === selectedProfileName)).join("")
    : `<div class="table-empty">
        <div>${profiles.length ? t("dashboard.noMatches") : t("empty.noProfiles")}</div>
        <button type="button" class="btn btn-ghost btn-sm" data-office-wizard-open>
          ${t("app.newProfile.office")}
        </button>
      </div>`;
  return `
    <section class="workspace">
      ${opsBarTemplate(profiles, health)}

      <section class="workbench">
        <div class="profile-list-panel">
          <div class="panel-toolbar">
            <div>
              <span class="eyebrow">${t("dashboard.table.eyebrow")}</span>
              <h2>${t("dashboard.table.title")}</h2>
            </div>
            <div class="profile-controls">
              <button type="button" class="btn btn-ghost btn-sm" data-office-wizard-open>
                ${t("app.newProfile.office")}
              </button>
              <label class="search-control" for="profile-search">
                <span class="sr-only">${t("dashboard.search")}</span>
                <input id="profile-search" class="field-input" type="search" value="${escapeAttr(profileQuery)}"
                       placeholder="${escapeAttr(t("dashboard.search"))}" autocomplete="off" />
              </label>
              <select id="profile-filter" class="field-select profile-filter" aria-label="${t("dashboard.filter")}">
                <option value="all" ${profileFilter === "all" ? "selected" : ""}>${t("dashboard.filter.all")}</option>
                <option value="running" ${profileFilter === "running" ? "selected" : ""}>${t("status.running")}</option>
                <option value="paused" ${profileFilter === "paused" ? "selected" : ""}>${t("status.paused")}</option>
                <option value="created" ${profileFilter === "created" ? "selected" : ""}>${t("status.created")}</option>
                <option value="exited" ${profileFilter === "exited" ? "selected" : ""}>${t("status.exited")}</option>
                <option value="absent" ${profileFilter === "absent" ? "selected" : ""}>${t("status.absent")}</option>
              </select>
            </div>
          </div>

          <div class="profile-table" role="table" aria-label="${t("dashboard.table.title")}">
            <div class="profile-table-head" role="row">
              <span role="columnheader">${t("dashboard.col.profile")}</span>
              <span role="columnheader">${t("dashboard.col.status")}</span>
              <span role="columnheader">${t("dashboard.col.ram")}</span>
              <span role="columnheader">${t("dashboard.col.endpoints")}</span>
              <span role="columnheader">${t("dashboard.col.bundles")}</span>
              <span role="columnheader">${t("dashboard.col.actions")}</span>
            </div>
            <div class="profile-table-body" role="rowgroup">${rows}</div>
          </div>
        </div>

        ${profileDetailTemplate(selected)}
      </section>
    </section>
  `;
}

function renderOfficeWizardScreen() {
  if (unbindOfficeWizard) unbindOfficeWizard();
  main.innerHTML = renderOfficeWizard(officeWizardState, { t, escapeHtml, escapeAttr });
  unbindOfficeWizard = bindOfficeWizard(main, {
    getState() {
      return officeWizardState;
    },
    dispatch(action) {
      officeWizardState = officeWizardReducer(officeWizardState, action);
      render();
    },
    onClose() {
      officeWizardOpen = false;
      render();
    },
    onStart() {
      startOfficeProvisioning();
    },
  });
  applyI18n(main);
}

function renderOfficeProgressWindowScreen() {
  if (!officeProgressWindowArgs) return;
  document.body.dataset.window = "office-progress";
  closeOfficeWizardBinding();
  main.innerHTML = renderOfficeProgressWindow(officeProgressWindowState, { t, escapeHtml, escapeAttr });
  applyI18n(main);
}

function closeOfficeWizardBinding() {
  if (!unbindOfficeWizard) return;
  unbindOfficeWizard();
  unbindOfficeWizard = null;
}

function openOfficeWizard() {
  officeWizardOpen = true;
  officeWizardState = initialOfficeWizardState();
  render();
  resumeOfficeWizardState(officeWizardState.profileName);
}

function officeProvisioningArgs(state) {
  return {
    name: String(state.profileName || "office").trim(),
    productId: state.productId || "O365ProPlusRetail",
    language: state.language || "pt-br",
    byolAccepted: Boolean(state.byolAccepted),
    resources: {
      ramGb: 8,
      cpuCores: 4,
      diskGb: 128,
      warningOverride: Boolean(state.warningOverride),
    },
  };
}

async function startOfficeProvisioning() {
  officeWizardState = officeWizardReducer(officeWizardState, {
    type: "set_status",
    status: "loading",
  });
  officeWizardState = officeWizardReducer(officeWizardState, { type: "set_step", step: "provisioning" });
  render();
  try {
    const response = await invoke("office_start_provisioning", {
      args: officeProvisioningArgs(officeWizardState),
    });
    officeWizardState = stateFromProvisioningResponse(officeWizardState, response);
    render();
  } catch (err) {
    const code = typeof err === "object" && err ? String(err.code || "office_start_failed") : "office_start_failed";
    officeWizardState = {
      ...officeWizardState,
      step: "provisioning",
      status: "error",
      errorCode: code,
      lastError: {
        code,
        message: formatErrorForToast(err),
        phase: err?.phase || "preflight",
        retryable: Boolean(err?.retryable),
      },
    };
    showErrorToast(err);
    render();
  }
}

async function resumeOfficeWizardState(profileName) {
  try {
    const response = await invoke("office_get_state", { args: { name: profileName } });
    officeWizardState = stateFromProvisioningResponse(officeWizardState, response);
    render();
  } catch {
    // Perfil novo ainda não tem estado persistido; o wizard continua em rascunho.
  }
}

// ─── Render dashboard ────────────────────────────────────────────────
let rendering = false;

function captureMainFocus() {
  const el = document.activeElement;
  if (!el || !main.contains(el) || !el.id) return null;
  return {
    id: el.id,
    start: typeof el.selectionStart === "number" ? el.selectionStart : null,
    end: typeof el.selectionEnd === "number" ? el.selectionEnd : null,
  };
}

function restoreMainFocus(snapshot) {
  if (!snapshot) return;
  const next = document.getElementById(snapshot.id);
  if (!next || !main.contains(next)) return;
  next.focus({ preventScroll: true });
  if (snapshot.start !== null && typeof next.setSelectionRange === "function") {
    next.setSelectionRange(snapshot.start, snapshot.end ?? snapshot.start);
  }
}

async function render(options = {}) {
  if (rendering) return;
  rendering = true;
  const focusSnapshot = captureMainFocus();
  try {
    if (officeProgressWindowArgs) {
      renderOfficeProgressWindowScreen();
      return;
    }
    if (officeWizardOpen) {
      renderOfficeWizardScreen();
      return;
    }
    closeOfficeWizardBinding();
    const [profiles, health] = await Promise.all([
      invoke("list_profiles"),
      loadHostHealth({ force: Boolean(options.forceHealth) }),
    ]);
    main.innerHTML = dashboardShell(profiles, health);
    applyI18n(main);
  } catch (e) {
    main.innerHTML = `
      <div class="empty">
        <div class="empty-illustration" style="color:var(--danger);border-color:var(--danger-soft)">
          ${ICO.alert}
        </div>
        <div class="empty-title">${t("empty.cliUnavailable")}</div>
        <div class="empty-desc">${escapeHtml(formatErrorForToast(e))}</div>
        <div class="empty-desc">${t("empty.cliUnavailable.hint")}</div>
      </div>`;
  } finally {
    rendering = false;
    restoreMainFocus(focusSnapshot);
  }
}

async function loadVersion() {
  try { versionEl.textContent = (await invoke("version")).trim(); }
  catch { versionEl.textContent = "CLI ausente"; }
}

// ─── Modal helpers ────────────────────────────────────────────────────
function openModal(id) { $(id).classList.add("open"); }
function closeModal(id) { $(id).classList.remove("open"); }

document.addEventListener("click", (ev) => {
  if (ev.target.matches("[data-close]") || ev.target.closest("[data-close]")) {
    const modal = ev.target.closest(".modal");
    if (modal) modal.classList.remove("open");
  }
});

document.addEventListener("click", (ev) => {
  if (!ev.target.closest("[data-office-wizard-open]")) return;
  openOfficeWizard();
});

function confirmDialog(title, message, yesLabel = "Confirmar") {
  return new Promise((resolve) => {
    $("#confirm-title").textContent = title;
    $("#confirm-message").textContent = message;
    const btn = $("#confirm-yes");
    btn.textContent = yesLabel;
    const modal = $("#modal-confirm");
    const onYes = () => { cleanup(); resolve(true); };
    const onNo  = () => { cleanup(); resolve(false); };
    function cleanup() {
      btn.removeEventListener("click", onYes);
      modal.querySelectorAll("[data-close]").forEach(el => el.removeEventListener("click", onNo));
      modal.classList.remove("open");
    }
    btn.addEventListener("click", onYes);
    modal.querySelectorAll("[data-close]").forEach(el => el.addEventListener("click", onNo));
    modal.classList.add("open");
  });
}

// Specialised remove dialog with an "also delete disk files" checkbox.
// Resolves to { confirmed: bool, deleteStorage: bool }. Checkbox starts
// checked so the default action mirrors what the user expects ("remove"
// means "remove everything") — they can uncheck to preserve disk files.
async function removeProfileDialog(name) {
  // Best-effort lookup of the profile's storage path. If it fails we
  // still show the dialog but the storage detail line stays generic.
  let storagePath = "";
  try {
    const cfg = await invoke("get_profile_config", { name });
    storagePath = (cfg && cfg.storage_dir) || "";
  } catch (_) { /* keep storagePath empty */ }

  return new Promise((resolve) => {
    const modal = $("#modal-remove-profile");
    const title = $("#remove-title");
    const msg = $("#remove-message");
    const detail = $("#remove-storage-detail");
    const checkbox = $("#remove-delete-storage");
    const confirmBtn = $("#remove-confirm");

    title.textContent = t("remove.title", { name }) || `Remove '${name}'?`;
    msg.textContent = t("remove.msg", { name }) || `Remove profile '${name}'?`;
    detail.textContent = storagePath
      ? t("remove.deleteStorage.path", { path: storagePath }) || storagePath
      : t("remove.deleteStorage.default") || "(default location under app data)";
    checkbox.checked = true;

    const onYes = () => {
      cleanup();
      resolve({ confirmed: true, deleteStorage: checkbox.checked });
    };
    const onNo = () => { cleanup(); resolve({ confirmed: false, deleteStorage: false }); };
    function cleanup() {
      confirmBtn.removeEventListener("click", onYes);
      modal.querySelectorAll("[data-close]").forEach(el => el.removeEventListener("click", onNo));
      modal.classList.remove("open");
    }
    confirmBtn.addEventListener("click", onYes);
    modal.querySelectorAll("[data-close]").forEach(el => el.addEventListener("click", onNo));
    modal.classList.add("open");
  });
}

// ─── Global floating menu (portaled to <body>) ────────────────────────
// A single .menu element lives at the root of <body>. Clicking "..." on a
// card populates it with that profile's actions and positions it next to
// the button. Avoids all containing-block/overflow pitfalls.
const floatingMenu = document.createElement("div");
floatingMenu.className = "menu";
floatingMenu.setAttribute("aria-label", t("card.menu"));
document.body.appendChild(floatingMenu);

let menuAnchor = null; // the "..." button that opened it
let menuProfile = null;
let menuProfileMeta = null; // { is_default, running, paused }

function renderFloatingMenu() {
  if (!menuProfileMeta) return;
  const { is_default, running, paused } = menuProfileMeta;
  floatingMenu.innerHTML = `
    <button type="button" class="menu-item" data-act="settings">${ICO.gear} ${t("card.menu.settings")}</button>
    <button type="button" class="menu-item" data-act="logs">${ICO.logs} ${t("card.menu.logs")}</button>
    <button type="button" class="menu-item" data-act="snapshots">${ICO.snapshot} ${t("card.menu.snapshots")}</button>
    <button type="button" class="menu-item" data-act="reapply">${ICO.reapply} ${t("card.menu.reapply")}</button>
    <button type="button" class="menu-item" data-act="restart">${ICO.reapply} ${t("card.menu.restart")}</button>
    <button type="button" class="menu-item" data-act="update-image">${ICO.download} ${t("card.menu.updateImage")}</button>
    ${!is_default ? `<button type="button" class="menu-item" data-act="default">${ICO.star} ${t("card.menu.makeDefault")}</button>` : ""}
    ${(running || paused) ? `<button type="button" class="menu-item danger" data-act="kill">${ICO.bolt} ${t("card.menu.kill")}</button>` : ""}
    <div class="menu-sep"></div>
    <button type="button" class="menu-item danger" data-act="remove">${ICO.trash} ${t("card.menu.remove")}</button>
  `;
}

function closeAllMenus() {
  floatingMenu.classList.remove("open");
  floatingMenu.style.top = floatingMenu.style.left = "";
  if (menuAnchor) menuAnchor.setAttribute("aria-expanded", "false");
  menuAnchor = null;
  menuProfile = null;
  menuProfileMeta = null;
}

function openFloatingMenu(anchorBtn) {
  const card = anchorBtn.closest(".card");
  if (!card) return;
  menuAnchor = anchorBtn;
  menuProfile = card.dataset.profile;
  menuProfileMeta = {
    is_default: card.dataset.isDefault === "1",
    running: card.dataset.state === "running",
    paused: card.dataset.state === "paused",
  };
  renderFloatingMenu();
  // Measure off-screen, then place.
  floatingMenu.style.top = "-9999px";
  floatingMenu.style.left = "-9999px";
  floatingMenu.classList.add("open");
  const btnRect = anchorBtn.getBoundingClientRect();
  const menuRect = floatingMenu.getBoundingClientRect();
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const gap = 8;
  let top = btnRect.bottom + gap;
  if (top + menuRect.height > vh - 8) top = btnRect.top - menuRect.height - gap;
  if (top < 8) top = 8;
  let left = btnRect.right - menuRect.width;
  if (left < 8) left = 8;
  if (left + menuRect.width > vw - 8) left = vw - menuRect.width - 8;
  floatingMenu.style.top = `${top}px`;
  floatingMenu.style.left = `${left}px`;
  anchorBtn.setAttribute("aria-expanded", "true");
  requestAnimationFrame(() => floatingMenu.querySelector(".menu-item")?.focus({ preventScroll: true }));
}

document.addEventListener("click", (ev) => {
  const toggleBtn = ev.target.closest('[data-act="menu"]');
  if (toggleBtn) {
    ev.stopPropagation();
    const wasOpen = floatingMenu.classList.contains("open") && menuAnchor === toggleBtn;
    closeAllMenus();
    if (!wasOpen) openFloatingMenu(toggleBtn);
    return;
  }
  // Close when clicking outside the menu
  if (!ev.target.closest(".menu")) closeAllMenus();
});
window.addEventListener("resize", closeAllMenus);
floatingMenu.addEventListener("keydown", (ev) => {
  const items = $$(".menu-item", floatingMenu);
  const currentIndex = items.indexOf(document.activeElement);
  if (ev.key === "ArrowDown") {
    ev.preventDefault();
    items[(currentIndex + 1 + items.length) % items.length]?.focus();
  } else if (ev.key === "ArrowUp") {
    ev.preventDefault();
    items[(currentIndex - 1 + items.length) % items.length]?.focus();
  } else if (ev.key === "Home") {
    ev.preventDefault();
    items[0]?.focus();
  } else if (ev.key === "End") {
    ev.preventDefault();
    items[items.length - 1]?.focus();
  }
});
document.addEventListener("scroll", (ev) => {
  if (!floatingMenu.classList.contains("open") || !menuAnchor) return;
  // Reposition rather than close — avoids the menu vanishing on focus-scroll.
  const btnRect = menuAnchor.getBoundingClientRect();
  const menuRect = floatingMenu.getBoundingClientRect();
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const gap = 8;
  let top = btnRect.bottom + gap;
  if (top + menuRect.height > vh - 8) top = btnRect.top - menuRect.height - gap;
  if (top < 8) top = 8;
  let left = btnRect.right - menuRect.width;
  if (left < 8) left = 8;
  floatingMenu.style.top = `${top}px`;
  floatingMenu.style.left = `${left}px`;
}, true);

// ─── Dashboard interactions ──────────────────────────────────────────
document.addEventListener("input", (ev) => {
  if (ev.target.matches("#profile-search")) {
    profileQuery = ev.target.value;
    render();
  }
});

document.addEventListener("change", (ev) => {
  if (ev.target.matches("#profile-filter")) {
    profileFilter = ev.target.value;
    render();
  }
});

document.addEventListener("click", (ev) => {
  const row = ev.target.closest("[data-select-profile]");
  if (!row || ev.target.closest("button, a, input, select, textarea")) return;
  selectedProfileName = row.dataset.profile;
  render();
});

document.addEventListener("click", async (ev) => {
  const btn = ev.target.closest("[data-refresh-health]");
  if (!btn) return;
  btn.disabled = true;
  try {
    await render({ forceHealth: true });
  } finally {
    btn.disabled = false;
  }
});

document.addEventListener("keydown", (ev) => {
  const row = ev.target.closest("[data-select-profile]");
  if (!row || ev.target !== row) return;
  if (ev.key !== "Enter" && ev.key !== " ") return;
  ev.preventDefault();
  selectedProfileName = row.dataset.profile;
  render();
});

// ─── Profile actions ─────────────────────────────────────────────────
document.addEventListener("click", async (ev) => {
  const btn = ev.target.closest("[data-act]");
  if (!btn) return;
  const act = btn.dataset.act;
  if (act === "menu") return; // handled above

  // If the action comes from the floating menu, use its captured profile.
  // Otherwise, fall back to the nearest .card (for direct card buttons like play).
  let name;
  if (btn.closest(".menu")) {
    name = menuProfile;
  } else {
    const card = btn.closest(".card");
    if (!card) return;
    name = card.dataset.profile;
  }
  if (!name) return;
  closeAllMenus();

  if (act === "remove") {
    const ok = await confirmDialog(
      t("confirm.remove.title", { name }),
      t("confirm.remove.msg", { name }),
      t("confirm.remove.yes"),
    );
    if (!ok) return;
  }
  if (act === "kill") {
    const ok = await confirmDialog(
      t("confirm.kill.title", { name }),
      t("confirm.kill.msg"),
      t("confirm.kill.yes"),
    );
    if (!ok) return;
  }
  if (act === "update-image") {
    const ok = await confirmDialog(
      t("confirm.update.title", { name }),
      t("confirm.update.msg"),
      t("confirm.update.yes"),
    );
    if (!ok) return;
  }
  if (act === "restart") {
    const ok = await confirmDialog(
      t("confirm.restart.title", { name }),
      t("confirm.restart.msg"),
      t("confirm.restart.yes"),
    );
    if (!ok) return;
  }
  if (act === "logs")     { openLogs(name); return; }
  if (act === "settings") { openSettings(name); return; }
  if (act === "reapply")  { openReapply(name); return; }
  if (act === "snapshots"){ openSnapshots(name); return; }

  btn.disabled = true;
  try {
    const cmdMap = {
      launch:  "launch_profile",
      pause:   "pause_profile",
      resume:  "resume_profile",
      stop:    "stop_profile",
      kill:    "kill_profile",
      remove:  "remove_profile",
      default: "set_default_profile",
      restart: "restart_profile",
      "update-image": "update_profile_image",
    };
    const cmd = cmdMap[act];
    if (!cmd) throw new Error("ação desconhecida: " + act);
    const res = await invoke(cmd, { name });
    showToast((typeof res === "string" ? res : res?.message) || `${act} em '${name}' ✓`, "success");
    setTimeout(render, 600);
  } catch (e) {
    if (act === "launch") {
      renderLaunchError(e);
    } else {
      showErrorToast(e);
    }
  } finally {
    btn.disabled = false;
  }
});

// ─── Logs modal ──────────────────────────────────────────────────────
let currentLogsProfile = null;
async function openLogs(name) {
  currentLogsProfile = name;
  $("#logs-profile").textContent = name;
  $("#logs-content").textContent = t("logs.loading");
  openModal("#modal-logs");
  await refreshLogs();
}
async function refreshLogs() {
  if (!currentLogsProfile) return;
  try {
    const out = await invoke("get_logs", { name: currentLogsProfile, tail: 300 });
    $("#logs-content").textContent = out || t("logs.empty");
    $("#logs-content").scrollTop = $("#logs-content").scrollHeight;
  } catch (e) {
    $("#logs-content").textContent = t("toast.genericError", { msg: formatErrorForToast(e) });
  }
}
$("#btn-refresh-logs").addEventListener("click", refreshLogs);

// ─── Snapshots modal ───────────────────────────────────────────────────
let currentSnapshotsProfile = null;

function defaultSnapshotName() {
  const d = new Date();
  const pad = n => String(n).padStart(2, "0");
  return `manual-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}`;
}

async function openSnapshots(name) {
  currentSnapshotsProfile = name;
  $("#snapshots-profile").textContent = name;
  $("#snapshot-name").value = defaultSnapshotName();
  $("#snapshots-list").textContent = t("snapshots.loading");
  openModal("#modal-snapshots");
  await refreshSnapshots();
}

async function refreshSnapshots() {
  if (!currentSnapshotsProfile) return;
  const list = $("#snapshots-list");
  try {
    const snaps = await invoke("list_snapshots", { name: currentSnapshotsProfile });
    if (!snaps.length) {
      list.innerHTML = `<div class="empty-inline">${t("snapshots.empty")}</div>`;
      return;
    }
    list.innerHTML = snaps.map(s => `
      <div class="snapshot-row">
        <div class="snapshot-meta">
          <div class="snapshot-name">${escapeHtml(s.name)}</div>
          <div class="snapshot-sub">${escapeHtml(s.created || "—")} · ${escapeHtml(s.size || "?")}</div>
        </div>
        <button type="button" class="btn btn-ghost btn-sm" data-snapshot-rollback="${escapeAttr(s.name)}">${t("snapshots.rollback")}</button>
      </div>
    `).join("");
  } catch (e) {
    list.innerHTML = `<div class="empty-inline">${escapeHtml(t("toast.genericError", { msg: formatErrorForToast(e) }))}</div>`;
  }
}

$("#btn-refresh-snapshots").addEventListener("click", refreshSnapshots);
$("#btn-create-snapshot").addEventListener("click", async () => {
  if (!currentSnapshotsProfile) return;
  const input = $("#snapshot-name");
  const snapshot = input.value.trim();
  if (!snapshot) return;
  const btn = $("#btn-create-snapshot");
  const original = btn.textContent;
  btn.disabled = true;
  btn.textContent = "…";
  try {
    const res = await invoke("create_snapshot", { name: currentSnapshotsProfile, snapshot });
    showToast(res?.message || t("snapshots.created"), "success");
    input.value = defaultSnapshotName();
    await refreshSnapshots();
  } catch (e) {
    showErrorToast(e);
  } finally {
    btn.disabled = false;
    btn.textContent = original;
  }
});

document.addEventListener("click", async (ev) => {
  const btn = ev.target.closest("[data-snapshot-rollback]");
  if (!btn || !currentSnapshotsProfile) return;
  const snapshot = btn.dataset.snapshotRollback;
  const ok = await confirmDialog(
    t("confirm.rollback.title", { name: currentSnapshotsProfile }),
    t("confirm.rollback.msg", { snapshot }),
    t("confirm.rollback.yes"),
  );
  if (!ok) return;
  btn.disabled = true;
  try {
    const res = await invoke("rollback_snapshot", { name: currentSnapshotsProfile, snapshot });
    showToast(res?.message || t("snapshots.restored"), "success");
    await refreshSnapshots();
    setTimeout(render, 800);
  } catch (e) {
    showErrorToast(e);
  } finally {
    btn.disabled = false;
  }
});

// ─── Reapply bundles modal ───────────────────────────────────────────
async function openReapply(name) {
  $("#reapply-profile").textContent = name;
  $("#reapply-status").textContent = t("reapply.status.generating");
  $("#reapply-paths").innerHTML = "";
  openModal("#modal-reapply");
  try {
    const res = await invoke("reapply_bundles", { name });
    $("#reapply-status").textContent = t("reapply.status.ready");
    $("#reapply-paths").innerHTML = `
      <div class="reapply-row"><div class="reapply-label">${t("reapply.label.hostPath")}</div><code>${escapeHtml(res.path)}</code></div>
      <div class="reapply-row"><div class="reapply-label">${t("reapply.label.winPath")}</div><code>${escapeHtml(res.windows_path)}</code></div>
      <div class="reapply-row"><div class="reapply-label">${t("reapply.label.cmd")}</div><pre>powershell -NoProfile -ExecutionPolicy Bypass -File "${escapeHtml(res.windows_path)}"</pre></div>
    `;
  } catch (e) {
    $("#reapply-status").textContent = t("reapply.error", { msg: formatErrorForToast(e) });
  }
}

// ─── Install modal ───────────────────────────────────────────────────
function gpuOptionsHTML(gpus, selectedBdf = "") {
  const none = `<option value="" ${!selectedBdf ? "selected" : ""}>${t("install.gpu.none")}</option>`;
  const opts = gpus.map(g => {
    const primarySuffix = g.is_primary_display ? t("install.gpu.disabled.primary") : "";
    const label = `${g.vendor} ${g.model} · ${g.bdf}${primarySuffix}`;
    const sel = g.bdf === selectedBdf ? "selected" : "";
    const disabled = g.is_primary_display ? "disabled" : "";
    return `<option value="${escapeAttr(g.bdf)}" ${sel} ${disabled}>${escapeHtml(label)}</option>`;
  }).join("");
  return none + opts;
}

/// Wire the single GPU select + setup-status block.
/// - deviceSelectId: <select> for the GPU BDF (empty = no GPU)
/// - statusId: <div> shown when a GPU is picked, with status + apply/revert button
/// - notesId: <div> for environmental notes (Optimus, IOMMU off)
/// - gpus: list from list_host_gpus
/// - initialBdf: BDF currently saved in this profile (or "" in install flow)
function wireGpuControls({ deviceSelectId, statusId, notesId, gpus, initialBdf = "" }) {
  const devSel = $(deviceSelectId);
  const status = $(statusId);
  const notes  = $(notesId);
  if (!devSel) return;

  devSel.innerHTML = gpuOptionsHTML(gpus, initialBdf);

  const renderEnvNotes = (g) => {
    if (!notes) return;
    if (!g) { notes.innerHTML = ""; return; }
    const lines = [];
    if (!g.vfio_capable) {
      lines.push(`<div class="gpu-note warn">${t("install.gpu.note.noIommu")}</div>`);
    }
    if (g.likely_optimus && (g.vendor || "").toLowerCase().includes("nvidia")) {
      lines.push(`<div class="gpu-note warn">${t("install.gpu.note.optimus")}</div>`);
    }
    notes.innerHTML = lines.join("");
  };

  const renderStatus = async () => {
    const bdf = devSel.value;
    if (!status) return;
    if (!bdf) {
      status.hidden = true;
      status.innerHTML = "";
      renderEnvNotes(null);
      return;
    }
    const g = gpus.find(x => x.bdf === bdf) || null;
    renderEnvNotes(g);
    status.hidden = false;
    status.innerHTML = `<div class="gpu-note">…</div>`;
    let st;
    try {
      st = await invoke("gpu_setup_status", { bdf });
    } catch (e) {
      status.innerHTML = `<div class="gpu-note warn">${escapeHtml(t("toast.genericError", { msg: formatErrorForToast(e) }))}</div>`;
      return;
    }
    paintStatus(status, bdf, st);
  };

  const paintStatus = (root, bdf, st) => {
    let kind, msgKey, btnLabel, btnAction;
    if (st.configured) {
      kind = "ok";
      msgKey = "install.gpu.status.ready";
      btnLabel = t("install.gpu.btn.revert");
      btnAction = "revert";
    } else if (st.needs_reboot) {
      kind = "warn";
      msgKey = "install.gpu.status.needReboot";
      btnLabel = t("install.gpu.btn.revert");
      btnAction = "revert";
    } else {
      kind = "warn";
      msgKey = "install.gpu.status.needSetup";
      btnLabel = t("install.gpu.btn.apply");
      btnAction = "apply";
    }
    root.innerHTML = `
      <div class="gpu-note ${kind}">${t(msgKey)}</div>
      <button type="button" class="btn btn-ghost btn-sm" data-gpu-action="${btnAction}" data-gpu-bdf="${escapeAttr(bdf)}">${btnLabel}</button>
    `;
    root.querySelector("[data-gpu-action]").addEventListener("click", async (ev) => {
      const btn = ev.currentTarget;
      const action = btn.dataset.gpuAction;
      const bdf2 = btn.dataset.gpuBdf;
      btn.disabled = true;
      const busyKey = action === "apply" ? "install.gpu.status.applying" : "install.gpu.status.reverting";
      root.innerHTML = `<div class="gpu-note">${t(busyKey)}</div>`;
      try {
        const cmd = action === "apply" ? "gpu_setup_apply" : "gpu_setup_revert";
        const st2 = await invoke(cmd, { bdf: bdf2 });
        paintStatus(root, bdf2, st2);
      } catch (e) {
        root.innerHTML = `<div class="gpu-note warn">${escapeHtml(t("toast.genericError", { msg: formatErrorForToast(e) }))}</div>`;
      }
    });
  };

  devSel.onchange = renderStatus;
  renderStatus();
}

function applyOsFamilyVisibility(form) {
  const family = (form.elements.image_family.value || "windows");
  form.querySelectorAll("[data-os]").forEach(el => {
    const allowed = el.dataset.os.split(/\s+/);
    const hide = !allowed.includes(family);
    // Use class + attribute so even rules that beat [hidden] still hide
    // the field. .os-hidden carries display:none !important.
    el.classList.toggle("os-hidden", hide);
    el.hidden = hide;
  });
  // Required-ness must follow visibility, otherwise hidden fields block submit.
  const versionEl = form.elements.version;
  if (versionEl) versionEl.required = (family === "windows");
  const bootEl = form.elements.boot;
  if (bootEl) bootEl.required = (family === "linux_distro");
  const isoEl = form.elements.iso_path;
  if (isoEl) isoEl.required = (family === "linux_iso");
  const passwordEl = form.elements.password;
  if (passwordEl) passwordEl.required = (family === "windows");
  const userEl = form.elements.user;
  if (userEl) userEl.required = (family === "windows");
}

async function openInstall(preselectFamily = null) {
  try {
    const [host, bundles, gpus, distros] = await Promise.all([
      invoke("host_info"),
      invoke("list_bundles"),
      invoke("list_host_gpus"),
      invoke("list_supported_distros"),
    ]);
    $("#ram-hint").textContent = `host: ${host.ram_gb}G`;
    $("#cpu-hint").textContent = `host: ${host.cpu_cores} cores`;
    const form = $("#form-install");
    form.reset();
    form.ram.value = Math.max(4, Math.floor(host.ram_gb / 3)) + "G";
    form.cpu.value = Math.max(2, Math.floor(host.cpu_cores / 2));

    // Pre-select OS family when caller specified one — and hide the OS
    // picker so the modal stays focused on a single VM type. The radios
    // remain in the DOM so applyOsFamilyVisibility / submit logic keep
    // working unchanged.
    const osPickerSection = $("#install-os-picker")?.closest(".section");
    if (preselectFamily) {
      const radio = form.querySelector(
        `input[name="image_family"][value="${preselectFamily}"]`
      );
      if (radio) radio.checked = true;
      if (osPickerSection) {
        osPickerSection.classList.add("os-hidden");
        osPickerSection.hidden = true;
      }
    } else if (osPickerSection) {
      osPickerSection.classList.remove("os-hidden");
      osPickerSection.hidden = false;
    }

    // Distro dropdown
    const bootSel = $("#install-boot");
    if (bootSel) {
      bootSel.innerHTML = distros
        .map(d => `<option value="${escapeAttr(d.id)}">${escapeHtml(d.label)}</option>`)
        .join("");
    }

    // ISO file picker
    const isoInput = $("#install-iso");
    const isoBtn = $("#install-iso-pick");
    if (isoBtn && !isoBtn.dataset.wired) {
      isoBtn.dataset.wired = "1";
      isoBtn.addEventListener("click", async () => {
        try {
          const path = await invoke("pick_iso_file");
          if (path) isoInput.value = path;
        } catch (e) {
          showErrorToast(e);
        }
      });
    }

    // Storage location picker (optional; empty = default in app data)
    const storageInput = $("#install-storage");
    const storagePickBtn = $("#install-storage-pick");
    const storageClearBtn = $("#install-storage-clear");
    if (storageInput) storageInput.value = "";
    if (storageClearBtn) storageClearBtn.hidden = true;
    if (storagePickBtn && !storagePickBtn.dataset.wired) {
      storagePickBtn.dataset.wired = "1";
      storagePickBtn.addEventListener("click", async () => {
        try {
          const path = await invoke("pick_storage_dir");
          if (!path) return;
          // Reject WSL drvfs mounts (/mnt/<letter>/...) up front — too slow
          // for VM disks. Backend also enforces this, but failing here gives
          // immediate feedback instead of failing at profile creation.
          if (/^\/mnt\/[a-zA-Z](\/|$)/.test(path)) {
            showToast(
              t("install.field.storage.drvfsRejected") ||
                "Esse drive (/mnt/...) é lento demais para discos de VM. Escolha uma pasta dentro do WSL, ex: /home/bruno/winbox-disks",
              "error",
            );
            storageInput.value = "";
            if (storageClearBtn) storageClearBtn.hidden = true;
            return;
          }
          storageInput.value = path;
          if (storageClearBtn) storageClearBtn.hidden = false;
        } catch (e) {
          showErrorToast(e);
        }
      });
    }
    if (storageClearBtn && !storageClearBtn.dataset.wired) {
      storageClearBtn.dataset.wired = "1";
      storageClearBtn.addEventListener("click", () => {
        storageInput.value = "";
        storageClearBtn.hidden = true;
      });
    }

    // OS family radio → toggles [data-os] sections
    form.querySelectorAll('input[name="image_family"]').forEach(r => {
      r.addEventListener("change", () => applyOsFamilyVisibility(form));
    });
    applyOsFamilyVisibility(form);

    const picker = $("#bundles-picker");
    picker.innerHTML = bundles.map(b => {
      const disabled = b.name === "essentials";
      return `
        <label class="bundle-card ${disabled ? "checked disabled" : ""}">
          <input type="checkbox" value="${escapeAttr(b.name)}" ${disabled ? "checked disabled" : ""} />
          <span class="check"></span>
          <span class="bundle-name">${escapeHtml(b.name)}</span>
          ${b.custom ? '<span class="bundle-tag">custom</span>' : ""}
        </label>`;
    }).join("");
    picker.querySelectorAll(".bundle-card:not(.disabled) input").forEach(inp => {
      inp.addEventListener("change", () => {
        inp.closest(".bundle-card").classList.toggle("checked", inp.checked);
      });
    });

    wireGpuControls({
      deviceSelectId: "#install-gpu",
      statusId: "#install-gpu-status",
      notesId: "#install-gpu-notes",
      gpus,
      initialBdf: "",
    });
  } catch (e) {
    showErrorToast(e);
    return;
  }
  openModal("#modal-install");
  applyI18n($("#modal-install"));
}

// New-profile dropdown: button toggles a small menu with one item per
// OS family. Clicking an item opens the install modal with that family
// pre-selected and the OS picker section hidden. Keyboard shortcut N
// (defined later) opens the menu too, then arrows / enter navigate.
(() => {
  const btn = $("#btn-install");
  const menu = $("#new-profile-menu");
  if (!btn || !menu) return;

  const positionMenu = () => {
    menu.style.top = "-9999px";
    menu.style.left = "-9999px";
    menu.classList.add("open");
    const btnRect = btn.getBoundingClientRect();
    const menuRect = menu.getBoundingClientRect();
    const gap = 6;
    let top = btnRect.bottom + gap;
    if (top + menuRect.height > window.innerHeight - 8) {
      top = btnRect.top - menuRect.height - gap;
    }
    let left = btnRect.left;
    if (left + menuRect.width > window.innerWidth - 8) {
      left = window.innerWidth - menuRect.width - 8;
    }
    menu.style.top = `${Math.max(8, top)}px`;
    menu.style.left = `${Math.max(8, left)}px`;
  };

  const openMenu = () => {
    positionMenu();
    btn.setAttribute("aria-expanded", "true");
    requestAnimationFrame(() =>
      menu.querySelector(".menu-item")?.focus({ preventScroll: true })
    );
  };

  const closeMenu = () => {
    menu.classList.remove("open");
    menu.style.top = menu.style.left = "";
    btn.setAttribute("aria-expanded", "false");
  };

  btn.addEventListener("click", (ev) => {
    ev.stopPropagation();
    if (menu.classList.contains("open")) {
      closeMenu();
    } else {
      openMenu();
    }
  });

  menu.addEventListener("click", (ev) => {
    const item = ev.target.closest(".menu-item");
    if (!item) return;
    const family = item.dataset.family;
    closeMenu();
    openInstall(family || null);
  });

  document.addEventListener("click", (ev) => {
    if (!menu.classList.contains("open")) return;
    if (menu.contains(ev.target) || btn.contains(ev.target)) return;
    closeMenu();
  });

  document.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape" && menu.classList.contains("open")) {
      closeMenu();
      btn.focus();
    }
  });
})();

$("#form-install").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const form = ev.target;
  const fd = new FormData(form);
  const family = (fd.get("image_family") || "windows").toString();
  const picked = $$("#bundles-picker input:checked:not(:disabled)")
    .map(el => el.value);
  const isWindows = family === "windows";
  const params = {
    name:        fd.get("name").toString().trim(),
    imageFamily: family,
    version:     isWindows ? (fd.get("version") || "").toString() : "",
    boot:        family === "linux_distro" ? (fd.get("boot") || "").toString() : "",
    isoPath:     family === "linux_iso" ? (fd.get("iso_path") || "").toString() : "",
    ram:         fd.get("ram").toString().trim(),
    cpu:         fd.get("cpu").toString(),
    disk:        fd.get("disk"),
    user:        isWindows ? (fd.get("user") || "").toString().trim() : "",
    password:    isWindows ? (fd.get("password") || "").toString() : "",
    language:    isWindows ? (fd.get("language") || "").toString() : "",
    region:      isWindows ? (fd.get("region") || "").toString().trim() : "",
    keyboard:    isWindows ? (fd.get("keyboard") || "").toString().trim() : "",
    bundles:     isWindows ? picked.join(",") : "",
    gpuBdf:      (fd.get("gpu") || "").toString(),
    storagePath: (fd.get("storage_path") || "").toString().trim(),
  };
  if (family === "linux_iso" && !params.isoPath) {
    showToast(t("toast.genericError", { msg: "Selecione o arquivo ISO." }), "error");
    return;
  }
  const btn = form.querySelector("button[type=submit]");
  const originalLabel = btn.textContent;
  btn.disabled = true;
  btn.textContent = "…";
  try {
    const res = await invoke("install_profile", { params });
    closeModal("#modal-install");
    showToast(res?.message || t("toast.profileCreated"), "success");
    setTimeout(render, 800);
  } catch (e) {
    showErrorToast(e);
  } finally {
    btn.disabled = false;
    btn.textContent = originalLabel;
  }
});

// ─── Settings modal + port forwarding ────────────────────────────────
let currentSettingsProfile = null;

function portRowHTML(host = "", cont = "", proto = "tcp") {
  const safeProto = proto === "udp" ? "udp" : "tcp";
  return `
    <div class="port-row">
      <input class="field-input" type="number" min="1" max="65535" placeholder="host" value="${escapeAttr(host)}" data-field="host" aria-label="Porta do host" />
      <span class="arrow">→</span>
      <input class="field-input" type="number" min="1" max="65535" placeholder="container" value="${escapeAttr(cont)}" data-field="container" aria-label="Porta do container" />
      <select class="field-select" data-field="proto" aria-label="Protocolo">
        <option value="tcp" ${safeProto === "tcp" ? "selected" : ""}>TCP</option>
        <option value="udp" ${safeProto === "udp" ? "selected" : ""}>UDP</option>
      </select>
      <button type="button" class="btn btn-icon" data-remove-port aria-label="Remover">
        ${ICO.close}
      </button>
    </div>`;
}

function parseExtraPorts(str) {
  if (!str) return [];
  return str.split(",").map(s => s.trim()).filter(Boolean).map(p => {
    const m = p.match(/^(\d+):(\d+)(?:\/(tcp|udp))?$/i);
    if (!m) return null;
    return { host: m[1], container: m[2], proto: (m[3] || "tcp").toLowerCase() };
  }).filter(Boolean);
}

function serializeExtraPorts() {
  return $$("#ports-list .port-row").map(row => {
    const host = row.querySelector('[data-field="host"]').value.trim();
    const cont = row.querySelector('[data-field="container"]').value.trim();
    const proto = row.querySelector('[data-field="proto"]').value;
    if (!host || !cont) return null;
    return `${host}:${cont}/${proto}`;
  }).filter(Boolean).join(",");
}

async function openSettings(name) {
  currentSettingsProfile = name;
  $("#settings-profile").textContent = name;
  try {
    const [cfg, host, gpus] = await Promise.all([
      invoke("get_profile_config", { name }),
      invoke("host_info"),
      invoke("list_host_gpus"),
    ]);
    const form = $("#form-settings");
    form.ram.value      = cfg.ram;
    form.cpu.value      = cfg.cpu;
    form.disk.value     = cfg.disk;
    form.user.value     = cfg.user;
    form.password.value = cfg.password;
    $("#settings-ram-hint").textContent = `host: ${host.ram_gb}G`;
    $("#settings-cpu-hint").textContent = `host: ${host.cpu_cores} cores`;
    $("#settings-version").textContent  = cfg.version;
    $("#settings-web").textContent      = `http://127.0.0.1:${cfg.web_port}`;
    $("#settings-rdp").textContent      = `127.0.0.1:${cfg.rdp_port}`;
    $("#settings-shared").textContent   = cfg.shared_dir;
    $("#settings-storage").textContent  = cfg.storage_dir;
    $("#settings-bundles").textContent  = cfg.bundles || "essentials";

    // Ports
    const ports = parseExtraPorts(cfg.extra_ports);
    const list = $("#ports-list");
    list.innerHTML = ports.length
      ? ports.map(p => portRowHTML(p.host, p.container, p.proto)).join("")
      : "";

    wireGpuControls({
      deviceSelectId: "#settings-gpu",
      statusId: "#settings-gpu-status",
      notesId: "#settings-gpu-notes",
      gpus,
      initialBdf: cfg.gpu_bdf || "",
    });
  } catch (e) {
    showErrorToast(e);
    return;
  }
  openModal("#modal-settings");
  applyI18n($("#modal-settings"));
}

$("#btn-add-port").addEventListener("click", () => {
  $("#ports-list").insertAdjacentHTML("beforeend", portRowHTML());
});
document.addEventListener("click", (ev) => {
  if (ev.target.closest("[data-remove-port]")) {
    ev.target.closest(".port-row").remove();
  }
});

$("#form-settings").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  if (!currentSettingsProfile) return;
  const form = ev.target;
  const fd = new FormData(form);
  const params = {
    name:       currentSettingsProfile,
    ram:        fd.get("ram")?.toString().trim() || null,
    cpu:        fd.get("cpu")?.toString().trim() || null,
    disk:       fd.get("disk")?.toString().trim() || null,
    user:       fd.get("user")?.toString().trim() || null,
    password:   fd.get("password")?.toString() || null,
    extraPorts: serializeExtraPorts(),
    gpuBdf:     (fd.get("gpu") || "").toString(),
    restart:    fd.get("restart") === "on",
  };
  const submit = form.querySelector("button[type=submit]");
  const origLabel = submit.textContent;
  submit.disabled = true;
  submit.textContent = "…";
  try {
    const res = await invoke("update_profile", { params });
    closeModal("#modal-settings");
    showToast(res?.message || t("settings.save"), "success");
    setTimeout(render, 800);
  } catch (e) {
    showErrorToast(e);
  } finally {
    submit.disabled = false;
    submit.textContent = origLabel;
  }
});

// ─── Atalhos + auto-refresh ─────────────────────────────────────────
$("#btn-refresh").addEventListener("click", () => render({ forceHealth: true }));
document.addEventListener("keydown", (e) => {
  if (e.target.matches("input, textarea, select")) return;
  if (e.key === "r" || e.key === "R") render();
  if (e.key === "n" || e.key === "N") openInstall();
  if (e.key === "Escape") {
    $$(".modal.open").forEach(m => m.classList.remove("open"));
    closeAllMenus();
  }
});

// Language switcher
const langSwitch = $("#lang-switch");
if (langSwitch) {
  langSwitch.value = getLocale();
  langSwitch.addEventListener("change", () => {
    setLocale(langSwitch.value);
    render(); // rerender cards with new strings
  });
}

// ─── Bootstrap wizard (first-run backend setup) ─────────────────────────
const BOOTSTRAP_STEP_FOR = {
  wsl2: "install_wsl",
  distro: "import_distro",
  docker: "start_docker",
  image: "pull_image",
};

function bootstrapCheckRow(c) {
  const icon = c.state === "ok" ? "✓" : (c.state === "missing" ? "!" : "?");
  const tone = c.state === "ok" ? "ok" : (c.state === "missing" ? "warn" : "muted");
  const step = BOOTSTRAP_STEP_FOR[c.id];
  const action = (c.state !== "ok" && step)
    ? `<button class="btn btn-primary btn-sm" data-bootstrap-step="${escapeAttr(step)}">${t("bootstrap.fix")}</button>`
    : "";
  return `
    <div class="bootstrap-row" data-tone="${tone}">
      <span class="bootstrap-state" data-tone="${tone}">${icon}</span>
      <span class="bootstrap-info">
        <strong>${escapeHtml(c.label)}</strong>
        <span class="bootstrap-detail">${escapeHtml(c.detail)}</span>
      </span>
      ${action}
    </div>`;
}

async function renderBootstrap() {
  const box = $("#bootstrap-checks");
  if (!box) return null;
  let status;
  try {
    status = await invoke("bootstrap_status");
  } catch (e) {
    box.innerHTML = `<div class="gpu-note warn">${escapeHtml(formatErrorForToast(e))}</div>`;
    return null;
  }
  box.innerHTML = (status.checks || []).map(bootstrapCheckRow).join("");
  return status;
}

async function runBootstrapStep(step, { force = false } = {}) {
  try {
    const outcome = await invoke("bootstrap_run_step", { step, force });
    if (outcome.outcome === "needs_confirmation") {
      const ok = await confirmDialog(
        t("bootstrap.confirm.title"),
        outcome.reason,
        t("bootstrap.confirm.yes"),
      );
      if (ok) return runBootstrapStep(step, { force: true });
      return;
    }
    if (outcome.outcome === "failed") {
      showToast(t("toast.genericError", { msg: outcome.error }), "error");
    } else {
      showToast(outcome.message || "ok", "success");
    }
  } catch (e) {
    showErrorToast(e);
  }
  await renderBootstrap();
}

async function checkBootstrapOnBoot() {
  // Only nag when the backend isn't ready. On Linux this is always ready.
  try {
    const status = await invoke("bootstrap_status");
    if (status && status.ready === false) {
      await renderBootstrap();
      openModal("#modal-bootstrap");
      applyI18n($("#modal-bootstrap"));
    }
  } catch { /* ignore — bootstrap is best-effort */ }
}

(() => {
  const recheck = $("#bootstrap-recheck");
  if (recheck) recheck.addEventListener("click", () => renderBootstrap());
  const box = $("#bootstrap-checks");
  if (box) {
    box.addEventListener("click", (ev) => {
      const btn = ev.target.closest("[data-bootstrap-step]");
      if (btn) runBootstrapStep(btn.dataset.bootstrapStep);
    });
  }
})();

// Initial i18n pass
applyI18n();

setInterval(render, 3000);

loadVersion();
render();
checkBootstrapOnBoot();
