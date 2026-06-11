const VERSION = "3.0.0";
const STORAGE_PREFIX = "axonos_nbg_v300_";
const LOGICAL_WIDTH = 1200;
const LOGICAL_HEIGHT = 640;
const BOUNDARY_X = 840;
const TICK_MS = 1000 / 60;
const MAX_CATCH_UP_TICKS = 8;
const ABI_VERSION = 3_000_000;
const EXPECTED_TICK_RATE = 60;
const EXPECTED_LANE_COUNT = 5;
const EXPECTED_BOUNDARY_X = 840;
const CONSENT_SCOPE_CONVERT = 1;
const CONSENT_SCOPE_RELEASE = 2;

export const WASM_EXPORTS = Object.freeze([
  "nbg_abi_version", "nbg_tick_rate", "nbg_lane_count", "nbg_boundary_x",
  "nbg_init", "nbg_daily_seed_low", "nbg_daily_seed_high", "nbg_tick", "nbg_action",
  "nbg_select_lane", "nbg_move_lane", "nbg_tick_value", "nbg_selected_lane",
  "nbg_trust", "nbg_risk", "nbg_integrity", "nbg_evidence", "nbg_review_gates",
  "nbg_raw_leaks", "nbg_raw_leak_limit", "nbg_score", "nbg_streak", "nbg_best_streak",
  "nbg_status", "nbg_terminal_reason", "nbg_feedback", "nbg_release_blockers",
  "nbg_release_ready", "nbg_consent_active", "nbg_consent_scope",
  "nbg_consent_expiry_tick", "nbg_state_hash_low", "nbg_state_hash_high",
  "nbg_entity_capacity", "nbg_active_entity_count", "nbg_entity_active", "nbg_entity_id",
  "nbg_entity_kind", "nbg_entity_lane", "nbg_entity_position", "nbg_entity_flags",
]);

const ACTION = Object.freeze({
  VALIDATE: 1,
  CONVERT: 2,
  QUARANTINE: 3,
  CONSENT: 4,
  EVIDENCE: 5,
  RELEASE: 6,
});

const MODE = Object.freeze({
  1: { name: "Guided", descriptor: "Learn the policy", seed: 58n },
  2: { name: "Standard", descriptor: "Canonical run" },
  3: { name: "Audit", descriptor: "Adversarial review" },
  4: { name: "Grand", descriptor: "Full boundary review" },
  5: { name: "Daily Seed", descriptor: "UTC deterministic run" },
});

const ENTITY = Object.freeze({
  1: { short: "RAW", name: "Raw frame", symbol: "R", tone: "danger", shape: "circle", action: "Quarantine" },
  2: { short: "ART", name: "Signal artifact", symbol: "A", tone: "danger", shape: "diamond", action: "Quarantine" },
  3: { short: "VALID", name: "Validated intent", symbol: "V", tone: "cyan", shape: "hex", action: "Convert" },
  4: { short: "?PKT", name: "Unknown packet", symbol: "?", tone: "neutral", shape: "circle", action: "Validate" },
  5: { short: "TYPED", name: "Typed intent", symbol: "T", tone: "safe", shape: "hex", action: "No action" },
  6: { short: "CONSENT", name: "Consent token", symbol: "C", tone: "gold", shape: "square", action: "Consent" },
  7: { short: "REVOKE", name: "Revoked consent", symbol: "×", tone: "danger", shape: "square", action: "Consent" },
  8: { short: "TRACE", name: "Evidence trace", symbol: "E", tone: "cyan", shape: "diamond", action: "Evidence" },
  9: { short: "SHA", name: "Checksum evidence", symbol: "#", tone: "gold", shape: "diamond", action: "Evidence" },
  10: { short: "CI", name: "CI validation", symbol: "✓", tone: "safe", shape: "square", action: "Evidence" },
  11: { short: "CLAIM", name: "Unsupported claim", symbol: "!", tone: "special", shape: "triangle", action: "Quarantine" },
  12: { short: "NO TRACE", name: "Untraceable claim", symbol: "!", tone: "special", shape: "triangle", action: "Quarantine" },
  13: { short: "ROADMAP", name: "Roadmap-as-fact claim", symbol: "!", tone: "special", shape: "triangle", action: "Quarantine" },
  14: { short: "STIM", name: "Stimulation command", symbol: "S", tone: "danger", shape: "octagon", action: "Quarantine immediately" },
});

const FEEDBACK = Object.freeze({
  0: ["BOUNDARY ONLINE", "Select a lane and apply the policy before a packet crosses.", "neutral"],
  1: ["INTENT VALIDATED", "The packet is classified. Convert only with consent and evidence.", "safe"],
  2: ["FALSE INTENT DETECTED", "Classification resolved the packet as an artifact. Quarantine it.", "warning"],
  3: ["TYPED INTENT CREATED", "Validated intent is now application-safe.", "safe"],
  4: ["CONVERSION BLOCKED", "Active scoped consent and at least L1 evidence are required.", "danger"],
  5: ["ENTITY QUARANTINED", "The unsafe object remained inside the sovereign boundary.", "safe"],
  6: ["CONSENT ACTIVE", "A scoped capability is valid until its deterministic expiry tick.", "safe"],
  7: ["CONSENT REVOKED", "The capability is invalid immediately. Conversion is blocked.", "warning"],
  8: ["EVIDENCE REGISTERED", "Review maturity increased.", "safe"],
  9: ["BOUNDARY SEALED", "Every mandatory release invariant passed.", "safe"],
  10: ["RELEASE BLOCKED", "One or more mandatory review invariants remain open.", "warning"],
  11: ["POLICY MISMATCH", "That action does not match the selected entity.", "danger"],
  12: ["NO TARGET", "The selected lane has no actionable entity.", "warning"],
  13: ["RAW SIGNAL LEAK", "Private signal crossed into the application zone.", "danger"],
  14: ["FAIL-CLOSED BREACH", "A stimulation command crossed the boundary. The run is unsafe.", "danger"],
  15: ["TYPED INTENT RELEASED", "Only typed intent reached the application layer.", "safe"],
  16: ["CONSENT EXPIRED", "The capability is stale and conversion is blocked.", "warning"],
  17: ["REVIEW WINDOW CLOSED", "The run ended before the boundary could be sealed.", "warning"],
});

const STATUS_NAME = Object.freeze(["Open", "Sealed", "Degraded", "Breached", "Unsafe"]);
const REASON_NAME = Object.freeze([
  "Active run",
  "Released",
  "Raw leak limit",
  "Stimulation crossed",
  "Integrity collapse",
  "Risk overflow",
  "Time expired",
  "Invariant violation",
]);

const BLOCKER = Object.freeze({
  1: "Trust below 90",
  2: "Risk above 20",
  4: "Integrity below 80",
  8: "Evidence below L2",
  16: "Review gates open",
  32: "Consent inactive",
  64: "Raw signal leaked",
  128: "Run already terminal",
  256: "Review window incomplete",
  512: "Active packets remain",
});

const PALETTE = Object.freeze({
  neutral: { stroke: "rgba(244,241,232,.72)", fill: "rgba(244,241,232,.055)", text: "#f4f1e8" },
  cyan: { stroke: "#79def5", fill: "rgba(121,222,245,.10)", text: "#bceffc" },
  gold: { stroke: "#d6b96b", fill: "rgba(214,185,107,.11)", text: "#f0dc9d" },
  safe: { stroke: "#78e6ad", fill: "rgba(120,230,173,.10)", text: "#b8f5d3" },
  danger: { stroke: "#ff7186", fill: "rgba(255,113,134,.11)", text: "#ffb6c1" },
  special: { stroke: "#a993ff", fill: "rgba(169,147,255,.11)", text: "#d4c9ff" },
});

export function clamp(value, minimum, maximum) {
  return Math.min(maximum, Math.max(minimum, value));
}

export function deriveUtcDateCode(date = new Date()) {
  return date.getUTCFullYear() * 10000 + (date.getUTCMonth() + 1) * 100 + date.getUTCDate();
}

export function formatHash(high, low) {
  const value = (BigInt(high >>> 0) << 32n) | BigInt(low >>> 0);
  return value.toString(16).padStart(16, "0");
}

export function blockerMessages(mask) {
  return Object.entries(BLOCKER)
    .filter(([bit]) => (mask & Number(bit)) !== 0)
    .map(([, message]) => message);
}

function safeStorageGet(key, fallback) {
  try {
    const value = localStorage.getItem(`${STORAGE_PREFIX}${key}`);
    return value === null ? fallback : value;
  } catch {
    return fallback;
  }
}

function safeStorageSet(key, value) {
  try {
    localStorage.setItem(`${STORAGE_PREFIX}${key}`, String(value));
  } catch {
    // Storage is optional. Gameplay remains fully functional without it.
  }
}

function clearNamespacedStorage() {
  try {
    const keys = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key?.startsWith(STORAGE_PREFIX)) keys.push(key);
    }
    keys.forEach((key) => localStorage.removeItem(key));
  } catch {
    // No-op when storage is unavailable.
  }
}

function freshSeed() {
  const values = new Uint32Array(2);
  crypto.getRandomValues(values);
  const seed = (BigInt(values[1]) << 32n) | BigInt(values[0]);
  return seed === 0n ? 1n : seed;
}

export function validateWasmAbi(exports) {
  const missing = WASM_EXPORTS.filter((name) => typeof exports[name] !== "function");
  if (missing.length > 0) throw new Error(`WASM ABI mismatch: missing ${missing.join(", ")}`);

  const metadata = [
    ["ABI version", exports.nbg_abi_version(), ABI_VERSION],
    ["tick rate", exports.nbg_tick_rate(), EXPECTED_TICK_RATE],
    ["lane count", exports.nbg_lane_count(), EXPECTED_LANE_COUNT],
    ["boundary coordinate", exports.nbg_boundary_x(), EXPECTED_BOUNDARY_X],
  ];
  const mismatches = metadata
    .filter(([, actual, expected]) => actual !== expected)
    .map(([label, actual, expected]) => `${label}: expected ${expected}, got ${actual}`);
  if (mismatches.length > 0) throw new Error(`WASM ABI metadata mismatch: ${mismatches.join("; ")}`);
  return exports;
}

async function instantiateWasm() {
  const wasmUrl = new URL("../pkg/neural_boundary_web.wasm", import.meta.url);
  const response = await fetch(wasmUrl, { cache: "no-cache", credentials: "same-origin" });
  if (!response.ok) throw new Error(`WASM request failed with HTTP ${response.status}`);

  let result;
  if (WebAssembly.instantiateStreaming) {
    try {
      result = await WebAssembly.instantiateStreaming(response.clone(), {});
    } catch {
      const bytes = await response.arrayBuffer();
      result = await WebAssembly.instantiate(bytes, {});
    }
  } else {
    const bytes = await response.arrayBuffer();
    result = await WebAssembly.instantiate(bytes, {});
  }
  return validateWasmAbi(result.instance.exports);
}

class ToneEngine {
  constructor() {
    this.context = null;
    this.muted = safeStorageGet("muted", "true") !== "false";
  }

  async ensureContext() {
    if (this.context) return this.context;
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) return null;
    this.context = new AudioContext();
    if (this.context.state === "suspended") await this.context.resume();
    return this.context;
  }

  async signal(kind) {
    if (this.muted) return;
    const context = await this.ensureContext();
    if (!context) return;
    const frequencies = { safe: 520, warning: 360, danger: 170, neutral: 420 };
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    const now = context.currentTime;
    oscillator.type = kind === "danger" ? "square" : "sine";
    oscillator.frequency.setValueAtTime(frequencies[kind] || 420, now);
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(0.035, now + 0.012);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.12);
    oscillator.connect(gain).connect(context.destination);
    oscillator.start(now);
    oscillator.stop(now + 0.13);
  }

  toggle() {
    this.muted = !this.muted;
    safeStorageSet("muted", this.muted);
    return this.muted;
  }
}

class NeuralBoundaryApp {
  constructor(wasm) {
    this.wasm = wasm;
    this.running = false;
    this.paused = false;
    this.mode = 1;
    this.seed = 58n;
    this.lastFrame = 0;
    this.accumulator = 0;
    this.raf = 0;
    this.lastFeedback = -1;
    this.lastAccessibleTick = -1;
    this.terminalPresented = false;
    this.resumeAfterHelp = false;
    this.reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
    this.tone = new ToneEngine();
    this.canvasTransform = { scale: 1, offsetX: 0, offsetY: 0, cssWidth: 1200, cssHeight: 640, dpr: 1 };
    this.cacheDom();
    this.bindEvents();
    this.restorePreferences();
    this.resizeCanvas();
    this.render();
  }

  cacheDom() {
    const byId = (id) => document.getElementById(id);
    this.dom = {
      intro: byId("intro-screen"),
      game: byId("game-screen"),
      runButton: byId("run-button"),
      bootState: byId("boot-state"),
      canvas: byId("game-canvas"),
      frame: byId("field-frame"),
      pauseOverlay: byId("pause-overlay"),
      pauseButton: byId("pause-button"),
      restartButton: byId("restart-button"),
      exitButton: byId("exit-button"),
      helpButton: byId("help-button"),
      helpDialog: byId("help-dialog"),
      resultDialog: byId("result-dialog"),
      resultRestart: byId("result-restart"),
      resultExit: byId("result-exit"),
      muteButton: byId("mute-button"),
      resetDataButton: byId("reset-data-button"),
      runModeLabel: byId("run-mode-label"),
      feedbackTitle: byId("feedback-title"),
      feedbackDetail: byId("feedback-detail"),
      liveRegion: byId("live-region"),
      accessibleEntities: byId("accessible-entities"),
      laneReadout: byId("lane-readout"),
      metricTrust: byId("metric-trust"),
      metricRisk: byId("metric-risk"),
      metricIntegrity: byId("metric-integrity"),
      metricEvidence: byId("metric-evidence"),
      metricScore: byId("metric-score"),
      barTrust: byId("bar-trust"),
      barRisk: byId("bar-risk"),
      barIntegrity: byId("bar-integrity"),
      runTick: byId("run-tick"),
      runLeaks: byId("run-leaks"),
      runStreak: byId("run-streak"),
      runConsent: byId("run-consent"),
      runActive: byId("run-active"),
      releaseControl: document.querySelector('[data-action="6"]'),
      gateElements: [...document.querySelectorAll("[data-gate]")],
      modeCards: [...document.querySelectorAll("[data-mode]")],
      actionButtons: [...document.querySelectorAll("[data-action]")],
      laneButtons: [...document.querySelectorAll("[data-lane-move]")],
    };
    this.ctx = this.dom.canvas.getContext("2d", { alpha: false });
  }

  restorePreferences() {
    const storedMode = Number.parseInt(safeStorageGet("mode", "1"), 10);
    if (MODE[storedMode]) this.selectMode(storedMode);
    this.updateMuteButton();
  }

  bindEvents() {
    this.dom.modeCards.forEach((card) => {
      card.addEventListener("click", () => this.selectMode(Number(card.dataset.mode)));
      card.addEventListener("keydown", (event) => this.handleModeKey(event, card));
    });
    this.dom.runButton.addEventListener("click", () => this.startRun());
    this.dom.actionButtons.forEach((button) => button.addEventListener("click", () => this.applyAction(Number(button.dataset.action))));
    this.dom.laneButtons.forEach((button) => button.addEventListener("click", () => this.moveLane(Number(button.dataset.laneMove))));
    this.dom.pauseButton.addEventListener("click", () => this.togglePause());
    this.dom.restartButton.addEventListener("click", () => this.restartRun());
    this.dom.exitButton.addEventListener("click", () => this.exitToIntro());
    this.dom.resultRestart.addEventListener("click", () => { this.dom.resultDialog.close(); this.restartRun(); });
    this.dom.resultExit.addEventListener("click", () => { this.dom.resultDialog.close(); this.exitToIntro(); });
    this.dom.helpButton.addEventListener("click", () => this.openHelp());
    this.dom.helpDialog.addEventListener("close", () => this.closeHelp());
    this.dom.resultDialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      this.exitToIntro();
    });
    this.dom.muteButton.addEventListener("click", () => { this.tone.toggle(); this.updateMuteButton(); });
    this.dom.resetDataButton.addEventListener("click", () => {
      clearNamespacedStorage();
      if (this.running) this.exitToIntro();
      this.selectMode(1);
      this.tone.muted = true;
      this.updateMuteButton();
      this.announce("Local Neural Boundary Game preferences and progress were reset.");
    });
    this.dom.canvas.addEventListener("pointerdown", (event) => this.selectLaneFromPointer(event));
    window.addEventListener("keydown", (event) => this.handleKey(event));
    window.addEventListener("resize", () => this.resizeCanvas(), { passive: true });
    document.addEventListener("visibilitychange", () => {
      this.lastFrame = performance.now();
      if (document.hidden) this.render();
    });
  }

  handleModeKey(event, card) {
    const key = event.key;
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(key)) return;
    event.preventDefault();
    const current = this.dom.modeCards.indexOf(card);
    let next = current;
    if (key === "Home") next = 0;
    else if (key === "End") next = this.dom.modeCards.length - 1;
    else if (key === "ArrowLeft" || key === "ArrowUp") next = (current - 1 + this.dom.modeCards.length) % this.dom.modeCards.length;
    else next = (current + 1) % this.dom.modeCards.length;
    const target = this.dom.modeCards[next];
    this.selectMode(Number(target.dataset.mode));
    target.focus();
  }

  selectMode(mode) {
    if (!MODE[mode]) return;
    this.mode = mode;
    safeStorageSet("mode", mode);
    this.dom.modeCards.forEach((card) => {
      const selected = Number(card.dataset.mode) === mode;
      card.classList.toggle("is-selected", selected);
      card.setAttribute("aria-checked", String(selected));
      card.tabIndex = selected ? 0 : -1;
    });
  }

  startRun() {
    if (this.mode === 5) {
      const dateCode = deriveUtcDateCode();
      const low = this.wasm.nbg_daily_seed_low(dateCode) >>> 0;
      const high = this.wasm.nbg_daily_seed_high(dateCode) >>> 0;
      this.seed = (BigInt(high) << 32n) | BigInt(low);
    } else if (MODE[this.mode].seed !== undefined) {
      this.seed = MODE[this.mode].seed;
    } else {
      this.seed = freshSeed();
    }

    const low = Number(this.seed & 0xffff_ffffn) >>> 0;
    const high = Number((this.seed >> 32n) & 0xffff_ffffn) >>> 0;
    this.wasm.nbg_init(low, high, this.mode, 2);
    this.running = true;
    this.paused = false;
    this.terminalPresented = false;
    this.lastFeedback = -1;
    this.lastAccessibleTick = -1;
    this.accumulator = 0;
    this.lastFrame = performance.now();
    this.dom.intro.hidden = true;
    this.dom.game.hidden = false;
    this.dom.pauseOverlay.hidden = true;
    this.dom.pauseButton.textContent = "Pause";
    this.dom.pauseButton.setAttribute("aria-pressed", "false");
    this.dom.runModeLabel.textContent = `${MODE[this.mode].name.toUpperCase()} RUN · SEED ${this.seed.toString()}`;
    safeStorageSet("last_seed", this.seed.toString());
    this.resizeCanvas();
    this.render();
    this.dom.pauseButton.focus({ preventScroll: true });
    cancelAnimationFrame(this.raf);
    this.raf = requestAnimationFrame((time) => this.frame(time));
  }

  restartRun() {
    if (!this.running) return this.startRun();
    const low = Number(this.seed & 0xffff_ffffn) >>> 0;
    const high = Number((this.seed >> 32n) & 0xffff_ffffn) >>> 0;
    this.wasm.nbg_init(low, high, this.mode, 2);
    this.paused = false;
    this.terminalPresented = false;
    this.lastFeedback = -1;
    this.accumulator = 0;
    this.lastFrame = performance.now();
    this.dom.pauseOverlay.hidden = true;
    this.dom.pauseButton.textContent = "Pause";
    this.dom.pauseButton.setAttribute("aria-pressed", "false");
    this.render();
    cancelAnimationFrame(this.raf);
    this.raf = requestAnimationFrame((time) => this.frame(time));
  }

  exitToIntro() {
    this.running = false;
    this.paused = false;
    cancelAnimationFrame(this.raf);
    if (this.dom.resultDialog.open) this.dom.resultDialog.close();
    this.dom.game.hidden = true;
    this.dom.intro.hidden = false;
    this.dom.runButton.focus({ preventScroll: true });
  }

  frame(time) {
    if (!this.running) return;
    const elapsed = clamp(time - this.lastFrame, 0, 250);
    this.lastFrame = time;

    if (!this.paused && !document.hidden && this.wasm.nbg_terminal_reason() === 0) {
      this.accumulator += elapsed;
      let ticks = 0;
      while (this.accumulator >= TICK_MS && ticks < MAX_CATCH_UP_TICKS) {
        this.wasm.nbg_tick(1);
        this.accumulator -= TICK_MS;
        ticks += 1;
      }
      if (ticks === MAX_CATCH_UP_TICKS) this.accumulator = 0;
    }

    this.render();
    if (this.wasm.nbg_terminal_reason() !== 0) this.presentResult();
    if (this.running && !this.terminalPresented) this.raf = requestAnimationFrame((next) => this.frame(next));
  }

  applyAction(action) {
    if (!this.running || this.paused || this.wasm.nbg_terminal_reason() !== 0) return;
    this.wasm.nbg_action(action);
    this.render();
    if (typeof navigator.vibrate === "function") {
      try {
        const feedback = this.wasm.nbg_feedback();
        if (feedback === 14 || feedback === 13) navigator.vibrate([35, 25, 60]);
        else if ([1, 3, 5, 6, 8, 9, 15].includes(feedback)) navigator.vibrate(18);
      } catch {
        // Haptics are optional and never affect deterministic state.
      }
    }
    if (this.wasm.nbg_terminal_reason() !== 0) this.presentResult();
  }

  moveLane(delta) {
    if (!this.running || this.paused) return;
    this.wasm.nbg_move_lane(delta);
    this.render();
  }

  togglePause() {
    if (!this.running || this.wasm.nbg_terminal_reason() !== 0) return;
    this.setPaused(!this.paused);
  }

  openHelp() {
    if (this.dom.helpDialog.open) return;
    this.resumeAfterHelp = this.running && !this.paused && this.wasm.nbg_terminal_reason() === 0;
    if (this.resumeAfterHelp) this.setPaused(true, false);
    this.dom.helpDialog.showModal();
  }

  closeHelp() {
    if (this.resumeAfterHelp && this.running && this.wasm.nbg_terminal_reason() === 0) {
      this.setPaused(false, false);
    }
    this.resumeAfterHelp = false;
  }

  setPaused(paused, announce = true) {
    this.paused = paused;
    this.lastFrame = performance.now();
    this.dom.pauseOverlay.hidden = !paused;
    this.dom.pauseButton.textContent = paused ? "Resume" : "Pause";
    this.dom.pauseButton.setAttribute("aria-pressed", String(paused));
    this.render();
    if (announce) this.announce(paused ? "Simulation paused." : "Simulation resumed.");
  }

  handleKey(event) {
    if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return;

    if (event.key === "Escape") {
      if (this.dom.helpDialog.open) this.dom.helpDialog.close();
      return;
    }
    if (event.key.toLowerCase() === "h" || event.key === "?") {
      if (this.dom.resultDialog.open) return;
      event.preventDefault();
      this.openHelp();
      return;
    }
    if (!this.running || this.dom.helpDialog.open || this.dom.resultDialog.open) return;

    const key = event.key.toLowerCase();
    if (key === "arrowup" || key === "w") { event.preventDefault(); this.moveLane(-1); }
    else if (key === "arrowdown" || key === "s") { event.preventDefault(); this.moveLane(1); }
    else if (key === "1") this.applyAction(ACTION.VALIDATE);
    else if (key === "2") this.applyAction(ACTION.CONVERT);
    else if (key === "3") this.applyAction(ACTION.QUARANTINE);
    else if (key === "4") this.applyAction(ACTION.CONSENT);
    else if (key === "5") this.applyAction(ACTION.EVIDENCE);
    else if (key === "enter") { event.preventDefault(); this.applyAction(ACTION.RELEASE); }
    else if (key === "p" || key === " ") { event.preventDefault(); this.togglePause(); }
    else if (key === "r") { event.preventDefault(); this.restartRun(); }
  }

  selectLaneFromPointer(event) {
    if (!this.running || this.paused) return;
    const rect = this.dom.canvas.getBoundingClientRect();
    const y = event.clientY - rect.top;
    const logicalY = (y - this.canvasTransform.offsetY) / this.canvasTransform.scale;
    const laneTop = 76;
    const laneHeight = 100;
    const lane = Math.floor((logicalY - laneTop) / laneHeight);
    if (lane >= 0 && lane < 5) {
      this.wasm.nbg_select_lane(lane);
      this.render();
    }
  }

  resizeCanvas() {
    const rect = this.dom.canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const dpr = clamp(window.devicePixelRatio || 1, 1, 2.5);
    const pixelWidth = Math.max(1, Math.round(rect.width * dpr));
    const pixelHeight = Math.max(1, Math.round(rect.height * dpr));
    if (this.dom.canvas.width !== pixelWidth || this.dom.canvas.height !== pixelHeight) {
      this.dom.canvas.width = pixelWidth;
      this.dom.canvas.height = pixelHeight;
    }
    const scale = Math.min(rect.width / LOGICAL_WIDTH, rect.height / LOGICAL_HEIGHT);
    this.canvasTransform = {
      scale,
      offsetX: (rect.width - LOGICAL_WIDTH * scale) / 2,
      offsetY: (rect.height - LOGICAL_HEIGHT * scale) / 2,
      cssWidth: rect.width,
      cssHeight: rect.height,
      dpr,
    };
    this.renderField();
  }

  readState() {
    return {
      tick: this.wasm.nbg_tick_value(),
      lane: this.wasm.nbg_selected_lane(),
      trust: this.wasm.nbg_trust(),
      risk: this.wasm.nbg_risk(),
      integrity: this.wasm.nbg_integrity(),
      evidence: this.wasm.nbg_evidence(),
      gates: this.wasm.nbg_review_gates(),
      leaks: this.wasm.nbg_raw_leaks(),
      leakLimit: this.wasm.nbg_raw_leak_limit(),
      score: this.wasm.nbg_score(),
      streak: this.wasm.nbg_streak(),
      bestStreak: this.wasm.nbg_best_streak(),
      status: this.wasm.nbg_status(),
      reason: this.wasm.nbg_terminal_reason(),
      feedback: this.wasm.nbg_feedback(),
      blockers: this.wasm.nbg_release_blockers(),
      ready: this.wasm.nbg_release_ready() === 1,
      consentActive: this.wasm.nbg_consent_active() === 1,
      consentScope: this.wasm.nbg_consent_scope(),
      consentExpiry: this.wasm.nbg_consent_expiry_tick(),
      activeEntities: this.wasm.nbg_active_entity_count(),
      hash: formatHash(this.wasm.nbg_state_hash_high(), this.wasm.nbg_state_hash_low()),
    };
  }

  readEntities() {
    const entities = [];
    const capacity = this.wasm.nbg_entity_capacity();
    for (let index = 0; index < capacity; index += 1) {
      if (this.wasm.nbg_entity_active(index) !== 1) continue;
      entities.push({
        id: this.wasm.nbg_entity_id(index),
        kind: this.wasm.nbg_entity_kind(index),
        lane: this.wasm.nbg_entity_lane(index),
        position: this.wasm.nbg_entity_position(index),
        flags: this.wasm.nbg_entity_flags(index),
      });
    }
    return entities;
  }

  render() {
    if (!this.dom.game.hidden) {
      const state = this.readState();
      const feedbackChanged = state.feedback !== this.lastFeedback;
      this.renderHud(state);
      this.renderField(state);
      this.renderFeedback(state, feedbackChanged);
      if (state.tick - this.lastAccessibleTick >= 30 || feedbackChanged) {
        this.renderAccessibleEntities();
        this.lastAccessibleTick = state.tick;
      }
      this.lastFeedback = state.feedback;
    }
  }

  renderHud(state) {
    this.dom.metricTrust.textContent = String(state.trust);
    this.dom.metricRisk.textContent = String(state.risk);
    this.dom.metricIntegrity.textContent = String(state.integrity);
    this.dom.metricEvidence.textContent = `L${state.evidence}`;
    this.dom.metricScore.textContent = String(state.score).padStart(6, "0");
    this.dom.barTrust.style.width = `${state.trust}%`;
    this.dom.barRisk.style.width = `${state.risk}%`;
    this.dom.barIntegrity.style.width = `${state.integrity}%`;
    this.dom.runTick.textContent = String(state.tick).padStart(4, "0");
    this.dom.runLeaks.textContent = `${state.leaks}/${state.leakLimit}`;
    this.dom.runStreak.textContent = String(state.streak);
    const consentScopes = [
      (state.consentScope & CONSENT_SCOPE_CONVERT) !== 0 ? "C" : "–",
      (state.consentScope & CONSENT_SCOPE_RELEASE) !== 0 ? "R" : "–",
    ].join("/");
    this.dom.runConsent.textContent = state.consentActive
      ? `Active ${consentScopes} · ${Math.max(0, state.consentExpiry - state.tick)}t`
      : "Inactive";
    this.dom.runActive.textContent = String(state.activeEntities);
    this.dom.laneReadout.textContent = String(state.lane + 1).padStart(2, "0");
    this.dom.releaseControl.classList.toggle("is-ready", state.ready);
    this.dom.releaseControl.querySelector("small").textContent = state.ready ? "All gates pass" : "Seal boundary";
    this.dom.gateElements.forEach((element) => {
      const bit = Number(element.dataset.gate);
      const passed = (state.gates & bit) !== 0;
      element.classList.toggle("is-passed", passed);
      element.classList.toggle("is-open", !passed);
      const label = element.dataset.gateLabel || element.querySelector("span")?.textContent || "Review gate";
      element.setAttribute("aria-label", `${label}: ${passed ? "passed" : "open"}`);
    });

    const controlsDisabled = this.paused || state.reason !== 0;
    this.dom.actionButtons.forEach((button) => { button.disabled = controlsDisabled; });
    this.dom.laneButtons.forEach((button) => { button.disabled = controlsDisabled; });
  }

  renderFeedback(state, feedbackChanged) {
    const entry = FEEDBACK[state.feedback] || FEEDBACK[0];
    this.dom.feedbackTitle.textContent = entry[0];
    if (state.feedback === 10) {
      const blockers = blockerMessages(state.blockers).filter((message) => message !== "Run already terminal");
      this.dom.feedbackDetail.textContent = blockers.length ? blockers.slice(0, 3).join(" · ") : entry[1];
    } else {
      this.dom.feedbackDetail.textContent = entry[1];
    }
    this.dom.feedbackTitle.style.color = {
      safe: "var(--state-safe)", warning: "var(--accent-gold)", danger: "var(--state-danger)", neutral: "var(--accent-cyan)",
    }[entry[2]];

    if (feedbackChanged && state.feedback !== 0) {
      this.announce(`${entry[0]}. ${this.dom.feedbackDetail.textContent}`);
      void this.tone.signal(entry[2]).catch(() => {});
    }
  }

  renderAccessibleEntities() {
    const entities = this.readEntities().sort((a, b) => b.position - a.position);
    this.dom.accessibleEntities.replaceChildren(...entities.map((entity) => {
      const item = document.createElement("li");
      const meta = ENTITY[entity.kind] || ENTITY[4];
      const distance = Math.max(0, BOUNDARY_X - entity.position);
      item.textContent = `Lane ${entity.lane + 1}: ${meta.name}; ${distance} units from boundary; required action ${meta.action}.`;
      return item;
    }));
  }

  renderField(state = this.readState()) {
    if (!this.ctx || !this.canvasTransform) return;
    const { dpr, scale, offsetX, offsetY, cssWidth, cssHeight } = this.canvasTransform;
    const ctx = this.ctx;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    ctx.fillStyle = "#05090d";
    ctx.fillRect(0, 0, cssWidth, cssHeight);
    ctx.save();
    ctx.translate(offsetX, offsetY);
    ctx.scale(scale, scale);

    const selectedY = 76 + state.lane * 100;
    ctx.fillStyle = "rgba(121,222,245,.035)";
    ctx.fillRect(0, selectedY, LOGICAL_WIDTH, 100);

    const signalGradient = ctx.createLinearGradient(0, 0, BOUNDARY_X, 0);
    signalGradient.addColorStop(0, "rgba(121,222,245,.018)");
    signalGradient.addColorStop(1, "rgba(121,222,245,.045)");
    ctx.fillStyle = signalGradient;
    ctx.fillRect(0, 0, BOUNDARY_X, LOGICAL_HEIGHT);
    ctx.fillStyle = "rgba(214,185,107,.025)";
    ctx.fillRect(BOUNDARY_X, 0, LOGICAL_WIDTH - BOUNDARY_X, LOGICAL_HEIGHT);

    ctx.strokeStyle = "rgba(255,255,255,.075)";
    ctx.lineWidth = 1;
    for (let lane = 0; lane <= 5; lane += 1) {
      const y = 76 + lane * 100;
      ctx.beginPath();
      ctx.moveTo(0, y + .5);
      ctx.lineTo(LOGICAL_WIDTH, y + .5);
      ctx.stroke();
    }

    for (let lane = 0; lane < 5; lane += 1) {
      const y = 126 + lane * 100;
      ctx.fillStyle = lane === state.lane ? "rgba(121,222,245,.92)" : "rgba(244,241,232,.30)";
      ctx.font = "700 10px ui-monospace, SFMono-Regular, Menlo, monospace";
      ctx.textAlign = "left";
      ctx.textBaseline = "middle";
      ctx.fillText(`0${lane + 1}`, 18, y);
    }

    ctx.strokeStyle = "rgba(214,185,107,.34)";
    ctx.lineWidth = 12;
    ctx.beginPath();
    ctx.moveTo(BOUNDARY_X, 42);
    ctx.lineTo(BOUNDARY_X, 608);
    ctx.stroke();
    ctx.strokeStyle = "#d6b96b";
    ctx.lineWidth = 1.5;
    ctx.setLineDash([6, 8]);
    ctx.beginPath();
    ctx.moveTo(BOUNDARY_X, 42);
    ctx.lineTo(BOUNDARY_X, 608);
    ctx.stroke();
    ctx.setLineDash([]);

    this.readEntities().forEach((entity) => this.drawEntity(ctx, entity));

    if (this.reducedMotion) {
      ctx.fillStyle = "rgba(244,241,232,.30)";
      ctx.font = "700 9px ui-monospace, SFMono-Regular, Menlo, monospace";
      ctx.textAlign = "right";
      ctx.fillText("REDUCED MOTION · MECHANICS UNCHANGED", LOGICAL_WIDTH - 18, LOGICAL_HEIGHT - 15);
    }

    ctx.restore();
  }

  drawEntity(ctx, entity) {
    const meta = ENTITY[entity.kind] || ENTITY[4];
    const palette = PALETTE[meta.tone] || PALETTE.neutral;
    const x = clamp(entity.position, 16, LOGICAL_WIDTH - 22);
    const y = 126 + entity.lane * 100;
    const radius = entity.kind === 14 ? 29 : 25;

    ctx.save();
    ctx.translate(x, y);
    ctx.fillStyle = palette.fill;
    ctx.strokeStyle = palette.stroke;
    ctx.lineWidth = entity.kind === 14 ? 2.5 : 1.5;
    this.entityPath(ctx, meta.shape, radius);
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = palette.text;
    ctx.font = "700 13px ui-monospace, SFMono-Regular, Menlo, monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(meta.symbol, 0, -1);
    ctx.restore();

    ctx.fillStyle = palette.text;
    ctx.font = "700 9px ui-monospace, SFMono-Regular, Menlo, monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(meta.short, x, y + radius + 8);
  }

  entityPath(ctx, shape, radius) {
    ctx.beginPath();
    if (shape === "circle") {
      ctx.arc(0, 0, radius, 0, Math.PI * 2);
      return;
    }
    const sides = { triangle: 3, square: 4, diamond: 4, hex: 6, octagon: 8 }[shape] || 6;
    const rotation = shape === "diamond" ? Math.PI / 4 : -Math.PI / 2;
    for (let index = 0; index < sides; index += 1) {
      const angle = rotation + index * (Math.PI * 2 / sides);
      const x = Math.cos(angle) * radius;
      const y = Math.sin(angle) * radius;
      if (index === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();
  }

  presentResult() {
    if (this.terminalPresented) return;
    this.terminalPresented = true;
    cancelAnimationFrame(this.raf);
    const state = this.readState();
    const grade = this.grade(state);
    document.getElementById("result-grade").textContent = grade.toUpperCase();
    document.getElementById("result-status").textContent = STATUS_NAME[state.status] || "Unknown";
    document.getElementById("result-reason").textContent = REASON_NAME[state.reason] || "Unknown";
    document.getElementById("result-score").textContent = String(state.score);
    document.getElementById("result-streak").textContent = String(state.bestStreak);
    document.getElementById("result-trust-risk").textContent = `${state.trust} / ${state.risk}`;
    document.getElementById("result-integrity").textContent = String(state.integrity);
    document.getElementById("result-evidence").textContent = `L${state.evidence}`;
    document.getElementById("result-leaks").textContent = String(state.leaks);
    document.getElementById("result-tick").textContent = String(state.tick);
    document.getElementById("result-hash").textContent = state.hash;
    document.getElementById("result-summary").textContent = state.status === 1
      ? "All mandatory invariants passed. The application layer received typed intent only."
      : `The run closed as ${STATUS_NAME[state.status]?.toLowerCase() || "unresolved"}: ${REASON_NAME[state.reason] || "unknown reason"}.`;

    const runs = Number.parseInt(safeStorageGet("runs", "0"), 10) || 0;
    const best = Number.parseInt(safeStorageGet("best_score", "0"), 10) || 0;
    safeStorageSet("runs", runs + 1);
    safeStorageSet("best_score", Math.max(best, state.score));
    this.dom.resultDialog.showModal();
    this.announce(`${grade}. ${document.getElementById("result-summary").textContent}`);
  }

  grade(state) {
    if (state.status === 1 && state.trust === 100 && state.risk === 0 && state.integrity >= 95 && state.evidence === 3) return "Sovereign";
    if (state.status === 1) return "Sealed";
    if (state.status === 2) return state.integrity >= 80 ? "Reviewable" : "Degraded";
    if (state.status === 3) return "Breached";
    return "Unsafe";
  }

  announce(message) {
    this.dom.liveRegion.textContent = "";
    requestAnimationFrame(() => { this.dom.liveRegion.textContent = message; });
  }

  updateMuteButton() {
    const muted = this.tone.muted;
    this.dom.muteButton.setAttribute("aria-pressed", String(muted));
    this.dom.muteButton.setAttribute("aria-label", muted ? "Sound muted" : "Sound enabled");
    this.dom.muteButton.querySelector("span").textContent = muted ? "◌" : "◉";
  }
}

async function boot() {
  const bootState = document.getElementById("boot-state");
  const runButton = document.getElementById("run-button");
  const fatal = document.getElementById("fatal-error");
  const fatalDetail = document.getElementById("fatal-detail");
  runButton.disabled = true;
  try {
    const wasm = await instantiateWasm();
    new NeuralBoundaryApp(wasm);
    bootState.textContent = `Rust/WASM core v${VERSION} verified · local-only`;
    bootState.classList.add("is-ready");
    runButton.disabled = false;
  } catch (error) {
    bootState.textContent = "Deterministic core unavailable";
    fatal.hidden = false;
    fatalDetail.textContent = error instanceof Error ? error.message : String(error);
    console.error(error);
  }
}

if (typeof document !== "undefined") {
  boot();
}
