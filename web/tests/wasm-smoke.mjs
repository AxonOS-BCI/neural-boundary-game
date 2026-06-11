import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { WASM_EXPORTS, validateWasmAbi } from "../app.js";

const wasmPath = process.argv[2] || "dist/pkg/neural_boundary_web.wasm";
const bytes = await readFile(wasmPath);
const { instance } = await WebAssembly.instantiate(bytes, {});
const api = validateWasmAbi(instance.exports);

assert.deepEqual(
  WASM_EXPORTS.filter((name) => typeof api[name] !== "function"),
  [],
  "browser and smoke-test ABI contracts diverged",
);

api.nbg_init(58, 0, 1, 2);
assert.equal(api.nbg_tick_value(), 0);
assert.equal(api.nbg_trust(), 60);
assert.equal(api.nbg_integrity(), 100);
assert.equal(api.nbg_selected_lane(), 2);
assert.ok(api.nbg_entity_capacity() >= 16);

api.nbg_select_lane(256);
assert.equal(api.nbg_selected_lane(), 4, "lane input must clamp instead of wrapping");
api.nbg_tick(60);
assert.equal(api.nbg_tick_value(), 60);
assert.ok(api.nbg_status() >= 0 && api.nbg_status() <= 4);
assert.ok(api.nbg_active_entity_count() <= api.nbg_entity_capacity());

const hash = (BigInt(api.nbg_state_hash_high() >>> 0) << 32n) | BigInt(api.nbg_state_hash_low() >>> 0);
assert.notEqual(hash, 0n);
console.log(`PASS: exact WASM ABI smoke · exports=${WASM_EXPORTS.length} · tick=${api.nbg_tick_value()} · hash=${hash.toString(16).padStart(16, "0")}`);
