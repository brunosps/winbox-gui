const HTML_ESCAPES = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

export function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, ch => HTML_ESCAPES[ch]);
}

export const escapeAttr = escapeHtml;

export function cssToken(value, fallback = "unknown") {
  const token = String(value ?? "").toLowerCase().replace(/[^a-z0-9_-]/g, "-");
  return token || fallback;
}
