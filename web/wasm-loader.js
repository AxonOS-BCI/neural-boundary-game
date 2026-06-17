// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Instantiates the flat WASM ABI v3 core and wraps it in a typed JS facade.
// The facade is read-only over engine state (§16.2): it never computes grade,
// score, or hash. Health is verified against §14.3 / §17.1.

const PRODUCT_VERSION_PACKED = 0x070300;
const ABI_VERSION = 3;

export const ACTION = {
  NONE: 0, AUDIT: 1, AUTHORIZE: 2, REVOKE: 3,
  QUARANTINE: 4, SEAL_VAULT: 5, THROTTLE: 6, RELEASE: 7,
};
export const GRADE = ["PENDING", "SEALED", "REVIEWABLE", "BREACHED", "UNSAFE"];
export const STATUS = ["RUNNING", "ENDED"];
export const RESULT = [
  "ACCEPTED", "REJECTED_COOLDOWN", "REJECTED_TERMINAL_STATE",
  "REJECTED_INVALID_FOR_EVENT", "REJECTED_RELEASE_LOCKED", "NO_OP",
];

export class VersionMismatch extends Error {}

export async function loadEngine(url) {
  let instance;
  try {
    const result = await WebAssembly.instantiateStreaming(fetch(url), {});
    instance = result.instance;
  } catch (_streamErr) {
    // Fall back when the server sends the wrong MIME type.
    const res = await fetch(url);
    if (!res.ok) throw new Error("wasm fetch failed: " + res.status);
    const bytes = await res.arrayBuffer();
    const result = await WebAssembly.instantiate(bytes, {});
    instance = result.instance;
  }
  const e = instance.exports;

  // §14.3 stale-WASM protection — fail closed on any mismatch.
  if (e.nbg_product_version_packed() !== PRODUCT_VERSION_PACKED) {
    throw new VersionMismatch("packed version mismatch");
  }
  if (e.nbg_abi_version() !== ABI_VERSION) {
    throw new VersionMismatch("abi version mismatch");
  }
  return new Engine(e);
}

class Engine {
  constructor(exports) {
    this.e = exports;
    this._dec = new TextDecoder();
  }
  _str(ptr, len) {
    if (!len) return "";
    const view = new Uint8Array(this.e.memory.buffer, ptr, len);
    return this._dec.decode(view);
  }

  // identity / health
  abiVersion() { return this.e.nbg_abi_version(); }
  versionPacked() { return this.e.nbg_product_version_packed(); }
  healthy() { return this.e.nbg_health_check() === PRODUCT_VERSION_PACKED; }
  tickRate() { return this.e.nbg_tick_rate(); }

  // lifecycle
  start(scenarioId, seed) {
    const hi = Number((BigInt(seed) >> 32n) & 0xffffffffn);
    const lo = Number(BigInt(seed) & 0xffffffffn);
    return this.e.nbg_new(scenarioId >>> 0, hi >>> 0, lo >>> 0) === 1;
  }
  reset() { return this.e.nbg_reset() === 1; }
  setScenario(id) { return this.e.nbg_set_scenario(id >>> 0) === 1; }
  initialized() { return this.e.nbg_is_initialized() === 1; }

  // tick / action
  setAction(code) { this.e.nbg_set_action(code >>> 0); }
  tick() { return RESULT[this.e.nbg_tick()] || "NO_OP"; }
  step(code) { return RESULT[this.e.nbg_step(code >>> 0)] || "NO_OP"; }

  // status / grade
  tickCount() { return this.e.nbg_tick_count(); }
  status() { return STATUS[this.e.nbg_status()] || "RUNNING"; }
  terminal() { return this.e.nbg_is_terminal() === 1; }
  grade() { return GRADE[this.e.nbg_grade()] || "PENDING"; }
  endReason() { return this._str(this.e.nbg_end_reason_label_ptr(), this.e.nbg_end_reason_label_len()); }
  score() { return this.e.nbg_score() | 0; }

  metrics() {
    const e = this.e;
    return {
      boundary_integrity: e.nbg_boundary_integrity(),
      consent_coherence: e.nbg_consent_coherence(),
      vault_integrity: e.nbg_vault_integrity(),
      cognitive_flow: e.nbg_cognitive_flow(),
      raw_leak_risk: e.nbg_raw_leak_risk(),
      stimulation_risk: e.nbg_stimulation_risk(),
      latency_pressure: e.nbg_latency_pressure(),
      audit_confidence: e.nbg_audit_confidence(),
    };
  }

  activeEvent() {
    const e = this.e;
    if (e.nbg_active_event_present() !== 1) return null;
    return {
      id: e.nbg_active_event_id(),
      kind: this._str(e.nbg_active_event_label_ptr(), e.nbg_active_event_label_len()),
      kindCode: e.nbg_active_event_kind(),
      severity: e.nbg_active_event_severity(),
      visibleRisk: e.nbg_active_event_visible_risk(),
      perceivedRisk: e.nbg_active_event_perceived_risk(),
      scope: e.nbg_active_event_scope(),
      requiresAudit: e.nbg_active_event_requires_audit() === 1,
      audited: e.nbg_active_event_audited() === 1,
      expiresAt: e.nbg_active_event_expires_at(),
    };
  }
  activeEventCount() { return this.e.nbg_active_event_count(); }

  defenses() {
    const e = this.e;
    return {
      scopes: e.nbg_permission_scopes(),
      permissionCount: e.nbg_permission_count(),
      sensitive: e.nbg_permission_sensitive() === 1,
      vaultSealed: e.nbg_vault_sealed() === 1,
      vaultCapacity: e.nbg_vault_capacity(),
      stimLevel: e.nbg_stimulation_level(),
      throttled: e.nbg_stimulation_throttled() === 1,
    };
  }

  release() {
    return {
      available: this.e.nbg_release_available() === 1,
      blockers: this.e.nbg_release_blockers(),
    };
  }
  lastActionResult() { return RESULT[this.e.nbg_last_action_result()] || "NO_OP"; }
  rejectedTotal() { return this.e.nbg_gate_rejected_total(); }

  counters() {
    const e = this.e;
    return {
      unresolvedCritical: e.nbg_counter_unresolved_critical(),
      unsafeActions: e.nbg_counter_unsafe_actions(),
      successfulAudits: e.nbg_counter_successful_audits(),
      correctRevocations: e.nbg_counter_correct_revocations(),
    };
  }

  hashHex() {
    const hi = this.e.nbg_state_hash_hi() >>> 0;
    const lo = this.e.nbg_state_hash_lo() >>> 0;
    return "0x" + hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
  }
  seedHex() {
    const hi = this.e.nbg_seed_hi() >>> 0;
    const lo = this.e.nbg_seed_lo() >>> 0;
    return hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
  }
  gradeLabel() { return this._str(this.e.nbg_grade_label_ptr(), this.e.nbg_grade_label_len()); }
  gradePublicLabel() { return this._str(this.e.nbg_grade_public_label_ptr(), this.e.nbg_grade_public_label_len()); }

  // scenario metadata
  scenarioId() { return this.e.nbg_scenario_id(); }
  scenarioCount() { return this.e.nbg_scenario_count(); }
  scenarioMaxTicks() { return this.e.nbg_scenario_max_ticks(); }
  scenarioName(id) { return this._str(this.e.nbg_scenario_name_ptr(id >>> 0), this.e.nbg_scenario_name_len(id >>> 0)); }
  scenarioObjective(id) { return this._str(this.e.nbg_scenario_objective_ptr(id >>> 0), this.e.nbg_scenario_objective_len(id >>> 0)); }
  scenarioDifficulty(id) {
    const DIFF = ["EASY", "MEDIUM", "HARD", "ELITE"];
    return DIFF[this.e.nbg_scenario_difficulty_by_id(id >>> 0)] || "—";
  }
}
