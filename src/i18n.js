// ─── Lightweight i18n ────────────────────────────────────────────────
// Supports two locales: en-US (default) and pt-BR. Translations live in
// window.I18N_BUNDLES which is populated by imported JSON-ish objects below.

const LS_KEY = "winbox:locale";
const SUPPORTED = ["en-US", "pt-BR"];

const bundles = {};

export function registerLocale(locale, dict) {
  bundles[locale] = dict;
}

let current = (localStorage.getItem(LS_KEY) || "en-US").trim();
if (!SUPPORTED.includes(current)) current = "en-US";

export function getLocale() { return current; }
export function listLocales() { return SUPPORTED.slice(); }

export function setLocale(locale) {
  if (!SUPPORTED.includes(locale)) return;
  current = locale;
  localStorage.setItem(LS_KEY, locale);
  applyAll();
}

/**
 * Look up a key in the current locale, fall back to en-US, fall back to key.
 * Supports simple `{name}` placeholders via the vars object.
 */
export function t(key, vars = {}) {
  const primary = bundles[current] || {};
  const fallback = bundles["en-US"] || {};
  let str = primary[key] ?? fallback[key] ?? key;
  for (const [k, v] of Object.entries(vars)) {
    str = str.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
  }
  return str;
}

/**
 * Apply i18n to the current DOM. Any element with `data-i18n="key"` has its
 * textContent replaced; `data-i18n-attr="attr:key,attr2:key2"` sets attrs.
 * Safe to call multiple times.
 */
export function applyAll(root = document) {
  root.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (key) el.textContent = t(key);
  });
  root.querySelectorAll("[data-i18n-attr]").forEach((el) => {
    const spec = el.getAttribute("data-i18n-attr");
    for (const pair of spec.split(",")) {
      const [attr, key] = pair.split(":").map((s) => s.trim());
      if (attr && key) el.setAttribute(attr, t(key));
    }
  });
  document.documentElement.lang = current;
}
