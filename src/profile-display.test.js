import { test } from "node:test";
import assert from "node:assert/strict";
import { modeMeta, primaryActionMeta, profileState } from "./profile-display.js";

const fakeT = (key) => key;
const icons = { play: "PLAY" };
const deps = { t: fakeT, icons };

test("modeMeta returns Web badge for any connect_mode", () => {
  assert.deepEqual(modeMeta({ connect_mode: "rdp" }), {
    label: "Web",
    className: "is-rdp",
  });
  assert.deepEqual(modeMeta({ connect_mode: "web_vnc" }), {
    label: "Web",
    className: "is-web_vnc",
  });
});

test("modeMeta defaults to web_vnc when connect_mode is missing", () => {
  assert.equal(modeMeta({}).label, "Web");
  assert.equal(modeMeta(undefined).label, "Web");
  assert.equal(modeMeta(null).label, "Web");
  assert.equal(modeMeta({}).className, "is-web_vnc");
});

test("profileState passes through known docker states", () => {
  for (const s of ["running", "paused", "exited", "created", "dead"]) {
    assert.equal(profileState({ status: s }), s);
  }
});

test("profileState defaults to 'absent' when status is missing", () => {
  assert.equal(profileState({}), "absent");
  assert.equal(profileState(undefined), "absent");
  assert.equal(profileState({ status: "" }), "absent");
});

test("primaryActionMeta picks 'launch'+connect for running", () => {
  const meta = primaryActionMeta({ status: "running" }, deps);
  assert.equal(meta.act, "launch");
  assert.equal(meta.label, "card.connect");
});

test("primaryActionMeta picks 'resume' for paused", () => {
  const meta = primaryActionMeta({ status: "paused" }, deps);
  assert.equal(meta.act, "resume");
  assert.equal(meta.label, "card.resume");
});

test("primaryActionMeta picks 'launch'+launch label for exited", () => {
  const meta = primaryActionMeta({ status: "exited" }, deps);
  assert.equal(meta.act, "launch");
  assert.equal(meta.label, "card.launch");
});

test("primaryActionMeta picks 'launch'+launch label for absent", () => {
  const meta = primaryActionMeta({}, deps);
  assert.equal(meta.act, "launch");
  assert.equal(meta.label, "card.launch");
});

test("primaryActionMeta always carries an icon from the injected map", () => {
  const meta = primaryActionMeta({ status: "running" }, deps);
  assert.equal(meta.icon, "PLAY");
});
