/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 *
 * Neural Boundary Game and AxonOS are protected intellectual property.
 * Commercial proprietary use requires an AxonOS Commercial License.
 */
// abi.js — typed wrapper over the flat WASM ABI (§26.1)

export let wasm = null;
let memory = null;

export async function loadWasm(url) {
  const resp = await fetch(url);
  const { instance } = await WebAssembly.instantiateStreaming(resp);
  wasm = instance.exports;
  memory = wasm.memory;
  if (!wasm.nbg_health_check || wasm.nbg_health_check() !== wasm.nbg_product_version_packed()) {
    throw new Error('WASM health-check failed: version mismatch');
  }
}

// Decode a null-terminated-ish string via ptr+len exports
function readStr(ptrFn, lenFn) {
  const ptr = wasm[ptrFn](), len = wasm[lenFn]();
  return new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
}

export const abi = {
  abiVersion:          () => wasm.nbg_abi_version(),
  // Product version: (5<<16)|(5<<8)|12 = 0x050512
  productVersionPacked:() => wasm.nbg_product_version_packed(),
  tickRate:            () => wasm.nbg_tick_rate(),
  laneCount:           () => wasm.nbg_lane_count(),
  entityCapacity:      () => wasm.nbg_entity_capacity(),
  boundaryX:           () => wasm.nbg_boundary_x(),
  schemaVersion:       () => readStr('nbg_schema_version_ptr', 'nbg_schema_version_len'),
  coreVersion:         () => readStr('nbg_core_version_ptr', 'nbg_core_version_len'),

  init(mode, seedHi, seedLo, difficulty) { return wasm.nbg_init(mode, seedHi, seedLo, difficulty); },
  reset()         { wasm.nbg_reset(); },
  step(ticks)     { return wasm.nbg_step(ticks); },
  applyAction(lane, actionId) { return wasm.nbg_apply_action(lane, actionId); },

  pause()         { wasm.nbg_pause(); },
  resume()        { wasm.nbg_resume(); },
  isPaused()      { return wasm.nbg_is_paused() !== 0; },
  selectLane(lane){ return wasm.nbg_select_lane(lane); },
  selectedLane()  { return wasm.nbg_selected_lane(); },

  phase()         { return wasm.nbg_phase(); },
  mode()          { return wasm.nbg_mode(); },
  tick()          { return wasm.nbg_tick(); },
  score()         { return wasm.nbg_score(); }, // BigInt (u64)
  trust()         { return wasm.nbg_trust(); },
  risk()          { return wasm.nbg_risk(); },
  integrity()     { return wasm.nbg_integrity(); },
  evidenceLevel() { return wasm.nbg_evidence_level(); },
  evidenceBits()  { return wasm.nbg_evidence_bits(); },
  gateMask()      { return wasm.nbg_gate_mask(); },
  blockerMask()   { return wasm.nbg_blocker_mask(); },
  consentScope()  { return wasm.nbg_consent_scope(); },
  consentEpoch()  { return wasm.nbg_consent_epoch(); },
  consentExpiresTick() { return wasm.nbg_consent_expires_tick(); },
  rawLeaks()      { return wasm.nbg_raw_leaks(); },
  combo()         { return wasm.nbg_combo(); },
  bestCombo()     { return wasm.nbg_best_combo(); },
  terminalStatus(){ return wasm.nbg_terminal_status(); },
  terminalReason(){ return wasm.nbg_terminal_reason(); },
  grade()         { return wasm.nbg_grade(); },
  stateHashHi()   { return wasm.nbg_state_hash_hi(); },
  stateHashLo()   { return wasm.nbg_state_hash_lo(); },
  stateHashHex()  {
    const hi = wasm.nbg_state_hash_hi() >>> 0;
    const lo = wasm.nbg_state_hash_lo() >>> 0;
    return '0x' + hi.toString(16).padStart(8,'0') + lo.toString(16).padStart(8,'0');
  },

  entityKind(slot) { return wasm.nbg_entity_kind(slot); },
  entityLane(slot) { return wasm.nbg_entity_lane(slot); },
  entityX(slot)    { return wasm.nbg_entity_x(slot); },
  entityFlags(slot){ return wasm.nbg_entity_flags(slot); },
};

// Mode codes (matches neural-boundary-core::Mode)
export const MODE = { GUIDED:1, STANDARD:2, AUDIT:3, GRAND:4, DAILY:5, PRIVACY_VAULT:6, KERNEL_TRIAL:7 };
export const MODE_NAME = {1:'GUIDED',2:'STANDARD',3:'AUDIT',4:'GRAND',5:'DAILY',6:'PRIVACY_VAULT',7:'KERNEL_TRIAL'};
// Action codes
export const ACTION = { VALIDATE:1, CONVERT:2, QUARANTINE:3, CONSENT:4, EVIDENCE:5, RELEASE:6 };
// Kind codes
export const KIND = { EMPTY:0, RAW_FRAME:1, ARTIFACT:2, UNKNOWN:3, CANDIDATE:4, VALIDATED:5,
  TYPED:6, GRANT:7, REVOKE:8, TRACE:9, CHECKSUM:10, CI:11, CLAIM:12, UNTRACE:13,
  ROADMAP:14, STIM:15, DEADLINE:16, VAULT_REC:17, RAW_EXPORT:18 };
// Gate bits
export const GATE = { PRIVACY:0x01, TYPING:0x02, CONSENT:0x04, EVIDENCE:0x08,
  DETERMINISM:0x10, VAULT:0x20, WCET:0x40, ALL:0x7F };
// Status codes
export const STATUS = { RUNNING:0, SEALED:1, BREACHED:2, UNSAFE:3, ABORTED:4, FATAL:5 };
// Grade codes
export const GRADE = { SOVEREIGN:0, SEALED:1, REVIEWABLE:2, DEGRADED:3, BREACHED:4, UNSAFE:5 };
// Difficulty
export const DIFF = { CALM:0, STANDARD:1, INTENSE:2 };

export const KIND_LABEL = {
  0:'EMPTY',1:'RAW_FRAME',2:'ARTIFACT',3:'UNKNOWN',4:'CANDIDATE',5:'VALIDATED',
  6:'TYPED',7:'GRANT',8:'REVOKE',9:'TRACE',10:'CHECKSUM',11:'CI_PROOF',
  12:'CLAIM',13:'UNTRACE',14:'ROADMAP',15:'STIM',16:'DEADLINE',17:'VAULT_REC',18:'RAW_EXPORT'
};
export const KIND_SYMBOL = {
  0:'·',1:'◉',2:'▒',3:'◌',4:'◇',5:'◈',6:'●',7:'⬡',8:'⬢',9:'▣',
  10:'▤',11:'▥',12:'△',13:'▽',14:'◭',15:'✕',16:'⧗',17:'◪',18:'⊘'
};
export const KIND_COLOR = {
  0:'#444', 1:'#ff7186', 2:'#ff7186', 3:'#a993ff', 4:'#79def5', 5:'#79def5',
  6:'#78e6ad', 7:'#78e6ad', 8:'#ff7186', 9:'#78e6ad', 10:'#78e6ad', 11:'#78e6ad',
  12:'#d6b96b', 13:'#d6b96b', 14:'#d6b96b', 15:'#ff4466', 16:'#d6b96b',
  17:'#ff7186', 18:'#ff7186'
};
export const GRADE_NAME = ['SOVEREIGN','SEALED','REVIEWABLE','DEGRADED','BREACHED','UNSAFE'];
export const STATUS_NAME = ['RUNNING','SEALED','BREACHED','UNSAFE','ABORTED','FATAL_RUNTIME'];
export const REASON_NAME = ['NONE','SUCCESS_RELEASE','TIMEOUT_UNSEALED','RISK_OVERFLOW',
  'INTEGRITY_COLLAPSE','RAW_LEAK_LIMIT','UNSAFE_STIMULATION_ESCAPE','DEADLINE_BREACH',
  'DETERMINISM_MISMATCH','REPLAY_SCHEMA_ERROR','WASM_INIT_FAILURE','USER_ABORT'];
