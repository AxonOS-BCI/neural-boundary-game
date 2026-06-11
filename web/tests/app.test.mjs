import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  WASM_EXPORTS,
  blockerMessages,
  clamp,
  deriveUtcDateCode,
  formatHash,
  validateWasmAbi,
} from "../app.js";

test("clamp enforces inclusive bounds", () => {
  assert.equal(clamp(-2, 0, 10), 0);
  assert.equal(clamp(7, 0, 10), 7);
  assert.equal(clamp(42, 0, 10), 10);
});

test("UTC date code is stable and timezone-independent", () => {
  assert.equal(deriveUtcDateCode(new Date("2026-06-11T23:59:59Z")), 20260611);
});

test("state hash combines unsigned halves", () => {
  assert.equal(formatHash(0x01234567, 0x89abcdef), "0123456789abcdef");
});

test("blocker mask resolves deterministic human diagnostics", () => {
  assert.deepEqual(
    blockerMessages(1 | 8 | 32 | 512),
    ["Trust below 90", "Evidence below L2", "Consent inactive", "Active packets remain"],
  );
});

test("WASM export list is unique and complete enough for the browser contract", () => {
  assert.equal(new Set(WASM_EXPORTS).size, WASM_EXPORTS.length);
  assert.ok(WASM_EXPORTS.length >= 40);
  assert.ok(WASM_EXPORTS.includes("nbg_abi_version"));
  assert.ok(WASM_EXPORTS.includes("nbg_active_entity_count"));
});

test("ABI validator rejects missing functions", () => {
  assert.throws(() => validateWasmAbi({}), /WASM ABI mismatch/);
});

test("ABI validator rejects stale metadata", () => {
  const api = Object.fromEntries(WASM_EXPORTS.map((name) => [name, () => 0]));
  assert.throws(() => validateWasmAbi(api), /metadata mismatch/);
});

test("ABI validator accepts exact metadata", () => {
  const api = Object.fromEntries(WASM_EXPORTS.map((name) => [name, () => 0]));
  api.nbg_abi_version = () => 3_000_000;
  api.nbg_tick_rate = () => 60;
  api.nbg_lane_count = () => 5;
  api.nbg_boundary_x = () => 840;
  assert.equal(validateWasmAbi(api), api);
});


test("consent scope constants remain explicit in the browser contract", async () => {
  const source = await readFile(new URL("../app.js", import.meta.url), "utf8");
  assert.match(source, /const CONSENT_SCOPE_CONVERT = 1;/);
  assert.match(source, /const CONSENT_SCOPE_RELEASE = 2;/);
  assert.match(source, /Active \$\{consentScopes\}/);
});

test("result modal blocks protocol modal keyboard nesting", async () => {
  const source = await readFile(new URL("../app.js", import.meta.url), "utf8");
  assert.match(source, /if \(this\.dom\.resultDialog\.open\) return;/);
});
