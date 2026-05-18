// Pure-function helpers for ProfileCard rendering. Kept dependency-free so
// node:test can exercise them without spinning up the DOM.

import { cssToken } from "./dom-utils.js";

// Maps the raw `status` field from the backend (which mirrors `docker ps`)
// to a stable family the UI can pattern-match against. Anything unknown is
// reported as "absent" so the UI never crashes on an unexpected state.
export function profileState(p) {
  return String((p && p.status) || "absent");
}

// Picks the connect-badge label + CSS class for a profile. Linux profiles
// (`connect_mode: "web_vnc"`) get "VNC"; anything else falls through to RDP.
export function modeMeta(p) {
  const mode = (p && p.connect_mode) || "rdp";
  return {
    label: mode === "rdp" ? "RDP" : "VNC",
    className: `is-${cssToken(mode)}`,
  };
}

// Chooses the primary-action button (act + i18n label + icon name) given
// the current state. The i18n + icon resolvers are injected so tests can
// run without importing the Tauri/DOM surface.
export function primaryActionMeta(p, { t, icons }) {
  const state = profileState(p);
  if (state === "running") {
    return { act: "launch", label: t("card.connect"), icon: icons.play };
  }
  if (state === "paused") {
    return { act: "resume", label: t("card.resume"), icon: icons.play };
  }
  return { act: "launch", label: t("card.launch"), icon: icons.play };
}
