// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Orchestrator: boot, engine load (fail-closed), scenario menu, deterministic
// 20 Hz tick loop, input, and the final report (§8.7). The Rust tick counter
// is the only clock; rAF merely paces calls to engine.tick().

import { loadEngine, ACTION, VersionMismatch } from "./wasm-loader.js";
import { BoundaryField } from "./renderer.js";
import { Hud } from "./hud.js";
import { buildMenu } from "./scenarios.js";
import {
  Prefs, initLiveRegion, announce, prefersReducedMotion, detectQuality,
} from "./accessibility.js";
import { Audio, cueForAction } from "./audio.js";

const WASM_URL = "./neural_boundary_web.wasm?v=7.3.0";
const KEY_TO_ACTION = {
  d: ACTION.AUDIT, a: ACTION.AUTHORIZE, r: ACTION.REVOKE, q: ACTION.QUARANTINE,
  s: ACTION.SEAL_VAULT, t: ACTION.THROTTLE, enter: ACTION.RELEASE,
};
const WHY = {
  RAW_LEAK_LIMIT: "Raw Leak Risk reached 100 after unresolved RAW_SIGNAL_EXPOSURE events. Seal the Privacy Vault earlier, or quarantine the exposure before it expires.",
  BOUNDARY_COLLAPSE: "Boundary Integrity fell to zero under sustained unresolved pressure. Audit and contain critical events before they expire.",
  UNSAFE_STIMULATION_ESCAPE: "Unsafe stimulation was authorized without a guardrail, or stimulation risk saturated. Throttle first; never authorize unsafe stimulation blind.",
  CONSENT_COLLAPSE: "Consent Coherence collapsed while a sensitive permission was active. Revoke stale sensitive scopes promptly.",
  VAULT_FAILURE: "The Privacy Vault failed while raw signal was exposed. Seal the vault before authorizing raw access.",
  TIMEOUT: "The session ended on time. Boundary held but was not sealed — release while stable next time.",
  SUCCESS_RELEASE: "You held sovereignty and released cleanly.",
};

const App = {
  engine: null, field: null, hud: null, audio: null,
  running: false, paused: false, reported: false,
  acc: 0, last: 0, currentScenario: 1, reducedMotion: false,
  prevGrade: "PENDING",
};

function randomSeed() {
  const buf = new Uint32Array(2);
  (window.crypto || window.msCrypto).getRandomValues(buf);
  return (BigInt(buf[0]) << 32n) | BigInt(buf[1]);
}

async function boot() {
  Prefs.cleanup();
  App.reducedMotion = Prefs.get(Prefs.key("reduced_motion"), prefersReducedMotion());
  const muted = Prefs.get(Prefs.key("audio_muted"), true);
  const quality = Prefs.get(Prefs.key("quality_profile"), detectQuality(App.reducedMotion));
  document.documentElement.dataset.quality = quality;
  document.documentElement.dataset.motion = App.reducedMotion ? "reduced" : "full";
  initLiveRegion(document);
  App.audio = new Audio(muted);
  wireToggles(muted);

  try {
    App.engine = await loadEngine(WASM_URL);
  } catch (err) {
    const kind = err instanceof VersionMismatch ? "mismatch"
      : (!navigator.onLine ? "offline" : "error");
    if (window.NBG_FALLBACK) window.NBG_FALLBACK.show(kind);
    return;
  }
  if (!App.engine.healthy()) {
    if (window.NBG_FALLBACK) window.NBG_FALLBACK.show("mismatch");
    return;
  }

  App.field = new BoundaryField(
    document.getElementById("main-canvas"), quality, App.reducedMotion
  );
  App.hud = new Hud(document);
  buildMenu(App.engine, document.getElementById("scenario-list"), startScenario);
  wireGameInput();
  wireReport();
  registerSW();
}

function wireToggles(muted) {
  const mb = document.getElementById("btn-mute");
  mb.setAttribute("aria-pressed", String(!muted));
  mb.textContent = "Audio: " + (muted ? "off" : "on");
  mb.addEventListener("click", () => {
    // aria-pressed="true" means audio is ON; flip it.
    const on = !(mb.getAttribute("aria-pressed") === "true");
    mb.setAttribute("aria-pressed", String(on));
    mb.textContent = "Audio: " + (on ? "on" : "off");
    App.audio.setMuted(!on);
    Prefs.set(Prefs.key("audio_muted"), !on);
    if (on) App.audio.unlock();
  });

  const motb = document.getElementById("btn-motion");
  motb.setAttribute("aria-pressed", String(App.reducedMotion));
  motb.textContent = "Motion: " + (App.reducedMotion ? "reduced" : "full");
  motb.addEventListener("click", () => {
    App.reducedMotion = !App.reducedMotion;
    motb.setAttribute("aria-pressed", String(App.reducedMotion));
    motb.textContent = "Motion: " + (App.reducedMotion ? "reduced" : "full");
    document.documentElement.dataset.motion = App.reducedMotion ? "reduced" : "full";
    if (App.field) App.field.setReducedMotion(App.reducedMotion);
    Prefs.set(Prefs.key("reduced_motion"), App.reducedMotion);
  });
}

function startScenario(id) {
  App.currentScenario = id;
  const seed = randomSeed();
  if (!App.engine.start(id, seed)) return;
  Prefs.set(Prefs.key("last_scenario"), id);
  document.getElementById("menu").hidden = true;
  document.getElementById("report-overlay").hidden = true;
  document.getElementById("game").hidden = false;
  App.audio.unlock();
  App.paused = false;
  App.reported = false;
  App.acc = 0;
  App.last = performance.now();
  App.prevGrade = "PENDING";
  App.hud.topbar(App.engine);
  if (!App.running) {
    App.running = true;
    requestAnimationFrame(loop);
  }
  announce("Scenario " + id + " started: " + App.engine.scenarioName(id));
}

function loop(now) {
  const e = App.engine;
  const step = 1000 / e.tickRate();
  App.acc += Math.min(now - App.last, 250); // clamp tab-stall
  App.last = now;
  if (!App.paused && !e.terminal()) {
    while (App.acc >= step && !e.terminal()) {
      e.tick(); // consumes any pending action set by input
      App.acc -= step;
    }
  } else {
    App.acc = 0;
  }

  const snap = {
    metrics: e.metrics(),
    defenses: e.defenses(),
    event: e.activeEvent(),
    grade: e.grade(),
    status: e.status(),
    release: e.release(),
  };
  App.field.draw(snap);
  App.hud.metrics(snap.metrics);
  App.hud.eventCard(snap.event, e.tickCount());
  App.hud.defenses(snap.defenses);
  App.hud.release(snap.release, e.terminal());
  App.hud.status(e);
  App.hud.missionHint(snap.event);
  App.hud.lastAction(e.lastActionResult());
  document.getElementById("meta-hash").textContent = e.hashHex();

  // grade-transition audio cues
  if (snap.grade !== App.prevGrade) {
    if (snap.grade === "BREACHED" || snap.grade === "UNSAFE") App.audio.play("BREACH");
    else if (snap.grade === "SEALED") App.audio.play("SEALED_RELEASE");
    App.prevGrade = snap.grade;
  }

  if (e.terminal() && !App.reported) {
    App.reported = true;
    showReport();
  }
  requestAnimationFrame(loop);
}

function doAction(code) {
  const e = App.engine;
  if (!e || e.terminal()) return;
  e.setAction(code);
  const cue = cueForAction(code);
  if (cue) App.audio.play(cue);
}

function wireGameInput() {
  document.querySelectorAll(".action, .release").forEach((btn) => {
    btn.addEventListener("click", () => doAction(Number(btn.dataset.action)));
  });
  document.getElementById("btn-pause").addEventListener("click", togglePause);
  window.addEventListener("keydown", (ev) => {
    if (document.getElementById("game").hidden) return;
    const key = ev.key.toLowerCase();
    if (key === " " || key === "spacebar") { ev.preventDefault(); togglePause(); return; }
    const action = KEY_TO_ACTION[key];
    if (action !== undefined) { ev.preventDefault(); doAction(action); }
  });
}

function togglePause() {
  App.paused = !App.paused;
  const b = document.getElementById("btn-pause");
  b.setAttribute("aria-pressed", String(App.paused));
  b.innerHTML = "<kbd>Space</kbd> " + (App.paused ? "Resume" : "Pause");
  announce(App.paused ? "Paused" : "Resumed");
}

function showReport() {
  const e = App.engine;
  const ov = document.getElementById("report-overlay");
  const grade = e.grade();
  const gEl = document.getElementById("report-grade");
  gEl.dataset.grade = grade;
  gEl.textContent = e.gradePublicLabel();
  document.getElementById("report-public").textContent =
    grade === "SEALED" ? "Sovereignty sealed." :
    grade === "REVIEWABLE" ? "Boundary held — reviewable." :
    grade === "UNSAFE" ? "Unsafe release blocked." : "Boundary breached.";
  const reason = e.endReason();
  document.getElementById("report-why").textContent =
    (grade === "BREACHED" ? "Boundary Breached. " : grade === "UNSAFE" ? "Unsafe. " : "")
    + (WHY[reason] || "");
  const c = e.counters();
  const stats = {
    Grade: grade, Score: e.score(), "End reason": reason,
    Scenario: e.scenarioId() + " · " + e.scenarioName(e.scenarioId()),
    Seed: e.seedHex(), "State hash": e.hashHex(),
    "Replay": "deterministic \u2713",
    "Successful audits": c.successfulAudits,
    "Correct revocations": c.correctRevocations,
    "Unsafe actions": c.unsafeActions,
    "Rejected actions": e.rejectedTotal(),
  };
  const dl = document.getElementById("report-stats");
  dl.innerHTML = "";
  for (const [k, v] of Object.entries(stats)) {
    const row = document.createElement("div");
    row.innerHTML = "<dt></dt><dd></dd>";
    row.querySelector("dt").textContent = k;
    row.querySelector("dd").textContent = String(v);
    dl.appendChild(row);
  }
  ov.hidden = false;
  document.querySelector(".report").focus();
  announce(e.gradePublicLabel() + ". Score " + e.score() + ". " + (WHY[reason] || ""));
}

function wireReport() {
  document.getElementById("btn-replay").addEventListener("click", () => startScenario(App.currentScenario));
  document.getElementById("btn-next").addEventListener("click", () => {
    const next = (App.currentScenario % App.engine.scenarioCount()) + 1;
    startScenario(next);
  });
  document.getElementById("btn-menu").addEventListener("click", () => {
    document.getElementById("report-overlay").hidden = true;
    document.getElementById("game").hidden = true;
    document.getElementById("menu").hidden = false;
  });
}

function registerSW() {
  if ("serviceWorker" in navigator) {
    window.addEventListener("load", () => {
      navigator.serviceWorker.register("./service-worker.js").catch(() => {});
    });
  }
}

boot();
