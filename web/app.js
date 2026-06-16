/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 *
 * Neural Boundary Game and AxonOS are protected intellectual property.
 * Commercial proprietary use requires an AxonOS Commercial License.
 */
// app.js — application state machine, RAF loop, input dispatch

import { abi, loadWasm, MODE, DIFF, ACTION, STATUS, GATE } from './abi.js';
import { update as hudUpdate, setFeedback, fillResult, fillBlocked, setModeBlurb } from './hud.js';
import { draw, fit, laneFromY, setDpr } from './render.js';
import { loadPrefs, savePrefs, saveBest, resetAll } from './storage.js';
import { announce, focusFirst, trapFocus } from './a11y.js';

// ── State ─────────────────────────────────────────────────────────────────────
let phase = 'landing'; // landing | running | paused | result
let activeModal = null;
let mode = MODE.STANDARD, difficulty = DIFF.STANDARD;
let seed = 0n, date = null;
let runCounter = 0;
let lastTs = 0, acc = 0;
const STEP_MS = 1000 / 60;
const MAX_STEPS = 5;
let reducedMotion = false;
let canvas, ctx;
let prevPauseState = false;

// ── Init ──────────────────────────────────────────────────────────────────────
async function boot() {
  canvas = document.getElementById('field');
  ctx = canvas.getContext('2d');
  setDpr(window.devicePixelRatio || 1);

  // Load WASM — fail closed
  try {
    await loadWasm('./neural_boundary_web.wasm');
    const health = abi.abiVersion();
    if (health !== 1) throw new Error('ABI version mismatch: ' + health);
    document.documentElement.dataset.nbgBooted = '1';
  } catch (err) {
    console.error('WASM init failed:', err);
    document.getElementById('boot-error').hidden = false;
    return;
  }

  // Load saved prefs
  const prefs = loadPrefs();
  mode = prefs.mode || MODE.STANDARD;
  difficulty = prefs.difficulty ?? DIFF.STANDARD;
  syncSelectors();

  // Reduced motion
  const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
  reducedMotion = mq.matches;
  mq.addEventListener('change', e => { reducedMotion = e.matches; });

  // Event delegation
  document.addEventListener('click', onDelegatedClick);
  document.addEventListener('keydown', onKey);
  canvas.addEventListener('click', onCanvasClick);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden' && phase === 'running') autoPause();
  });

  // CTA focus
  document.getElementById('cta-run')?.focus();
  requestAnimationFrame(frame);
}

// ── RAF loop ─────────────────────────────────────────────────────────────────
function frame(ts) {
  requestAnimationFrame(frame);
  if (lastTs === 0) lastTs = ts;
  const dt = Math.min(ts - lastTs, 250);
  lastTs = ts;

  if (phase === 'running' && !activeModal) {
    acc += dt;
    let steps = 0;
    while (acc >= STEP_MS && steps < MAX_STEPS) {
      abi.step(1);
      acc -= STEP_MS;
      steps++;
      if (abi.terminalStatus() !== STATUS.RUNNING) { onTerminal(); break; }
    }
    if (steps === MAX_STEPS) acc = 0;
  }

  if (phase !== 'landing') {
    hudUpdate();
    draw(canvas, ctx, reducedMotion);
  }
}

// ── Run lifecycle ─────────────────────────────────────────────────────────────
function startRun(reuseSeed) {
  if (!reuseSeed || !seed) {
    if (mode === MODE.DAILY) {
      const now = new Date();
      const y = now.getUTCFullYear(), m = now.getUTCMonth()+1, d = now.getUTCDate();
      date = `${y}-${String(m).padStart(2,'0')}-${String(d).padStart(2,'0')}`;
      // daily_seed computed same way as Rust core
      seed = computeDailySeed(date);
    } else {
      date = null;
      seed = mixSeed(BigInt(Date.now()), BigInt(++runCounter));
    }
  }

  const seedHi = Number((seed >> 32n) & 0xFFFFFFFFn);
  const seedLo = Number(seed & 0xFFFFFFFFn);
  const rc = abi.init(mode, seedHi, seedLo, difficulty);
  if (rc !== 0) { console.error('nbg_init failed:', rc); return; }

  acc = 0; lastTs = 0;
  closeModal();
  document.getElementById('game-shell').hidden = false;
  document.getElementById('landing-screen').hidden = true;
  document.getElementById('hud-mode-chip').textContent =
    ['','GUIDED','STANDARD','AUDIT','GRAND RUN','DAILY','PRIVACY VAULT','KERNEL TRIAL'][mode] +
    ' · ' + ['CALM','STANDARD','INTENSE'][difficulty];
  document.getElementById('seed-chip').textContent =
    mode === 5 && date ? `DAILY ${date}` : `SEED ${seed.toString(16).padStart(16,'0').toUpperCase()}`;
  phase = 'running';
  setFeedback('RUN STARTED · hold the boundary', 'ok');
  announce(`Run started in ${['','Guided','Standard','Audit','Grand','Daily','Privacy Vault','Kernel Trial'][mode]} mode.`);
}

function onTerminal() {
  phase = 'result';
  const status = abi.terminalStatus(), grade = abi.grade();
  saveBest(mode, Number(abi.score()), grade);
  fillResult(seed, date, mode, difficulty);
  openModal('ov-result');
  announce(`Run ended. Grade ${['SOVEREIGN','SEALED','REVIEWABLE','DEGRADED','BREACHED','UNSAFE'][grade]}. Score ${abi.score()}.`);
}

function autoPause() {
  if (phase === 'running') {
    phase = 'paused';
    setFeedback('AUTO-PAUSED · browser hidden', 'warn');
    document.getElementById('btn-pause').textContent = 'RESUME';
    openModal('ov-pause');
  }
}

function togglePause() {
  if (phase === 'running') {
    phase = 'paused';
    document.getElementById('btn-pause').textContent = 'RESUME';
    openModal('ov-pause');
  } else if (phase === 'paused') {
    phase = 'running'; lastTs = 0;
    document.getElementById('btn-pause').textContent = 'PAUSE';
    closeModal();
  }
}

function toMenu() {
  phase = 'landing';
  document.getElementById('game-shell').hidden = true;
  document.getElementById('landing-screen').hidden = false;
  closeModal();
  syncSelectors();
}

// ── Input ─────────────────────────────────────────────────────────────────────
function onKey(e) {
  if (activeModal) {
    const el = document.getElementById(activeModal);
    if (el) trapFocus(el, e);
    if (e.key === 'Escape' && activeModal !== 'ov-result') { e.preventDefault(); closeModal(); }
    if (e.key === 'h' || e.key === 'H') { if (activeModal === 'ov-help') closeModal(); }
    return;
  }
  const running = phase === 'running';
  switch (e.key) {
    case 'ArrowUp': case 'w': case 'W':
      if (running) { e.preventDefault(); moveLane(-1); } break;
    case 'ArrowDown': case 's': case 'S':
      if (running) { e.preventDefault(); moveLane(1); } break;
    case '1': if (running) applyAction(ACTION.VALIDATE); break;
    case '2': if (running) applyAction(ACTION.CONVERT); break;
    case '3': if (running) applyAction(ACTION.QUARANTINE); break;
    case '4': if (running) applyAction(ACTION.CONSENT); break;
    case '5': if (running) applyAction(ACTION.EVIDENCE); break;
    case 'Enter': if (running) { e.preventDefault(); applyAction(ACTION.RELEASE); } break;
    case ' ': e.preventDefault(); if (running || phase === 'paused') togglePause(); break;
    case 'p': case 'P': if (running || phase === 'paused') togglePause(); break;
    case 'r': case 'R': if (phase === 'result') startRun(true); break;
    case 'h': case 'H': openModal('ov-help'); if (running) { phase='paused'; } break;
    case 'Escape': if (phase === 'paused') togglePause(); break;
  }
}

function onCanvasClick(e) {
  const lane = laneFromY(canvas, e.clientY);
  if (lane >= 0) abi.selectLane(lane);
}

function onDelegatedClick(e) {
  const el = e.target.closest('[data-cmd],[data-act],[data-mode],[data-difficulty]');
  if (!el) return;
  const cmd = el.dataset.cmd, act = el.dataset.act, md = el.dataset.mode, diff = el.dataset.difficulty;
  if (cmd) handleCmd(cmd);
  else if (act) applyAction(parseInt(act));
  else if (md) { mode = parseInt(md); savePrefs(mode, difficulty); syncSelectors(); }
  else if (diff !== undefined) { difficulty = parseInt(diff); savePrefs(mode, difficulty); syncSelectors(); }
}

function handleCmd(cmd) {
  switch (cmd) {
    case 'start': case 'new-run': startRun(false); break;
    case 'rerun': startRun(true); break;
    case 'pause': togglePause(); break;
    case 'resume': if (phase==='paused') togglePause(); break;
    case 'restart': startRun(false); break;
    case 'menu': toMenu(); break;
    case 'help': openModal('ov-help'); break;
    case 'close-modal': closeModal(); break;
    case 'lane-up': moveLane(-1); break;
    case 'lane-down': moveLane(1); break;
    case 'reset-data': openModal('ov-reset'); break;
    case 'reset-confirm': resetAll(); closeModal(); announce('Local data cleared.'); syncSelectors(); break;
  }
}

function moveLane(delta) {
  const cur = abi.selectedLane();
  const next = Math.max(0, Math.min(4, cur + delta));
  abi.selectLane(next);
}

function applyAction(actionId) {
  if (phase !== 'running') return;
  const lane = abi.selectedLane();
  abi.applyAction(lane, actionId);
  const status = abi.terminalStatus();
  if (status !== STATUS.RUNNING) { onTerminal(); return; }
  // Show blocker if release was blocked
  if (actionId === ACTION.RELEASE && status === STATUS.RUNNING) {
    fillBlocked();
    openModal('ov-blocked');
  }
}

// ── Modals ────────────────────────────────────────────────────────────────────
function openModal(id) {
  if (activeModal && activeModal !== id) closeModal();
  activeModal = id;
  const el = document.getElementById(id);
  if (!el) return;
  el.hidden = false;
  el.setAttribute('aria-hidden', 'false');
  focusFirst(el);
}

function closeModal() {
  if (!activeModal) return;
  const el = document.getElementById(activeModal);
  if (el) { el.hidden = true; el.setAttribute('aria-hidden', 'true'); }
  if (activeModal === 'ov-pause' && phase === 'paused') {
    phase = 'running'; lastTs = 0;
    document.getElementById('btn-pause').textContent = 'PAUSE';
  }
  activeModal = null;
}

// ── Selectors ─────────────────────────────────────────────────────────────────
function syncSelectors() {
  document.querySelectorAll('[data-mode]').forEach(el => {
    el.setAttribute('aria-checked', el.dataset.mode == mode ? 'true' : 'false');
  });
  document.querySelectorAll('[data-difficulty]').forEach(el => {
    el.setAttribute('aria-checked', el.dataset.difficulty == difficulty ? 'true' : 'false');
  });
  setModeBlurb(mode);
}

// ── Daily seed (mirrors Rust core daily_seed fn) ──────────────────────────────
function computeDailySeed(isoDate) {
  // FNV-1a 64 over "NBG|5.5.12|YYYY-MM-DD|DAILY" then one xorshift64star round
  const OFFSET = 0xcbf29ce484222325n, PRIME = 0x100000001b3n;
  const MOD = 0xFFFFFFFFFFFFFFFFn;
  let h = OFFSET;
  const feed = b => { h = ((h ^ BigInt(b)) * PRIME) & MOD; };
  for (const c of 'NBG|5.5.12|' + isoDate + '|DAILY') feed(c.charCodeAt(0));
  if (h === 0n) h = 0x3001n;
  // One xorshift64star round
  let x = h;
  x ^= x >> 12n; x &= MOD;
  x ^= (x << 25n) & MOD;
  x ^= x >> 27n;
  x = (x * 0x2545f4914f6cdd1dn) & MOD;
  return x === 0n ? 0x3001n : x;
}

// ── Seed mix ───────────────────────────────────────────────────────────────────
function mixSeed(nowMs, counter) {
  const PRIME1 = 0xbf58476d1ce4e5b9n, PRIME2 = 0x94d049bb133111ebn, MOD = 0xFFFFFFFFFFFFFFFFn;
  let x = (nowMs ^ (counter << 48n) ^ 0x9e3779b97f4a7c15n) & MOD;
  x ^= x >> 30n; x = (x * PRIME1) & MOD;
  x ^= x >> 27n; x = (x * PRIME2) & MOD;
  x ^= x >> 31n;
  return x === 0n ? 0x3001n : x;
}

document.addEventListener('DOMContentLoaded', boot);
