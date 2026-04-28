import test from "node:test";
import assert from "node:assert/strict";

import { cssToken, escapeAttr, escapeHtml } from "./dom-utils.js";

test("escapeHtml escapes executable markup", () => {
  assert.equal(
    escapeHtml(`<img src=x onerror="alert('x')">&`),
    "&lt;img src=x onerror=&quot;alert(&#39;x&#39;)&quot;&gt;&amp;",
  );
});

test("escapeAttr uses the same escaping rules", () => {
  assert.equal(escapeAttr(`a" b'`), "a&quot; b&#39;");
});

test("cssToken keeps only class-safe characters", () => {
  assert.equal(cssToken("web vnc<script>"), "web-vnc-script-");
  assert.equal(cssToken(""), "unknown");
});
