/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 */
// hud.js — DOM metrics mirror, feedback grammar, result screen

import { abi, GATE, GATE as G, GRADE_NAME, STATUS_NAME, REASON_NAME, MODE_NAME, KIND_LABEL } from './abi.js';
import { announce } from './a11y.js';

const GATE_NAMES = ['PRIVACY','TYPING','CONSENT','EVIDENCE','DETERMINISM','VAULT','WCET'];
const MODE_BLURB = {
  1: 'A 60-second coaching run. Each boundary action demonstrated once before it matters.',
  2: 'Canonical run. Gate all 7 reviews. Trust≥750, Risk≤250, Integrity≥750, leaks=0 for SEALED.',
  3: 'Stricter evidence (L3), consent ceiling and tighter risk budget. One raw leak ends the run.',
  4: 'Four phases. Signal Integrity → Consent & Evidence → Release Under Pressure → Boundary Review. Release blocked until phase 4.',
  5: 'Deterministic daily seed from "NBG|5.5.12|YYYY-MM-DD|DAILY". Same UTC date, same world, no backend.',
  6: 'Raw frames, vault records and export requests are heavy. Contain the vault or it compromises permanently.',
  7: 'Missed deadline hazards accumulate; three misses end the run. Constant time-pressure.'
};

const $ = id => document.getElementById(id);

function bar(id, v, max = 1000) {
  const el = $(id);
  if (el) el.style.width = Math.max(0, Math.min(100, v / max * 100)) + '%';
}

export function update() {
  const tick = abi.tick(), tickRate = abi.tickRate();
  const elapsed = Math.floor(tick / tickRate);
  const mm = String(Math.floor(elapsed / 60)).padStart(2,'0');
  const ss = String(elapsed % 60).padStart(2,'0');

  const trust = abi.trust(), risk = abi.risk(), integrity = abi.integrity();
  const score = abi.score(); // BigInt
  const combo = abi.combo();
  const ev = abi.evidenceLevel();
  const evBits = abi.evidenceBits();
  const gates = abi.gateMask();
  const leaks = abi.rawLeaks();
  const consent = abi.consentScope();
  const expiry = abi.consentExpiresTick();
  const phase = abi.phase();
  const mode = abi.mode();

  setText('hud-trust', trust);
  setText('hud-risk', risk);
  setText('hud-integrity', integrity);
  setText('hud-score', score.toString());
  setText('hud-combo', combo > 0 ? `×${combo}` : '—');
  setText('hud-evidence', ['L0','L1','L2','L3'][ev] + ' (' + evBitStr(evBits) + ')');
  setText('hud-leaks', leaks);
  setText('hud-time', `${mm}:${ss}`);
  bar('bar-trust', trust); bar('bar-risk', risk); bar('bar-integrity', integrity);

  const consentEl = $('hud-consent');
  if (consentEl) {
    if (consent === 0) { consentEl.textContent = 'INACTIVE'; consentEl.className = 'hud-v'; }
    else {
      const rem = Math.max(0, expiry - tick);
      consentEl.textContent = `ACTIVE ${Math.ceil(rem / abi.tickRate())}s`;
      consentEl.className = 'hud-v consent-on';
    }
  }

  // Gate row
  const gateEl = $('hud-gates');
  if (gateEl) {
    let html = '';
    for (let g = 0; g < 7; g++) {
      const pass = (gates >> g) & 1;
      html += `<span class="gate ${pass?'pass':'fail'}" title="${GATE_NAMES[g]}">${GATE_NAMES[g][0]}</span>`;
    }
    gateEl.innerHTML = html;
  }

  // Phase (Grand only)
  const phaseEl = $('hud-phase');
  if (phaseEl) {
    const PHASE_NAME = ['Signal Integrity','Consent & Evidence','Release Under Pressure','Boundary Review'];
    phaseEl.hidden = mode !== 4;
    if (mode === 4) phaseEl.textContent = `PHASE ${phase+1}: ${PHASE_NAME[phase]||''}`;
  }
}

function setText(id, v) { const el = $(id); if (el) el.textContent = v; }

function evBitStr(bits) {
  return (bits & 1 ? 'T' : '·') + (bits & 2 ? 'C' : '·') + (bits & 4 ? 'I' : '·');
}

export function setFeedback(text, tone = 'neutral') {
  const el = $('feedback');
  if (!el) return;
  el.textContent = text;
  el.className = `feedback ${tone}`;
}

export function setModeBlurb(mode) {
  const el = $('mode-blurb');
  if (el) el.textContent = MODE_BLURB[mode] || '';
}

export function fillResult(seed, date, mode, difficulty) {
  const tick = abi.tick(), status = abi.terminalStatus(), reason = abi.terminalReason();
  const grade = abi.grade(), gates = abi.gateMask();
  const trust = abi.trust(), risk = abi.risk(), integrity = abi.integrity();
  const score = abi.score();

  const TITLES = ['SOVEREIGN BOUNDARY','BOUNDARY SEALED','UNDER REVIEW','DEGRADED BOUNDARY','BOUNDARY BREACHED','UNSAFE TERMINATION'];
  const CLASSES = ['sovereign','sealed','reviewable','degraded','breached','unsafe'];
  const BODIES = [
    'Raw signal stayed private. Applications received typed intent only. All 7 gates passed.',
    'Boundary released. Not all excellence thresholds were met, but the run sealed.',
    'Horizon reached. The boundary held but the release was never completed.',
    'Some gates passed, but the run degraded before a clean seal.',
    'The boundary was breached. Contain hazards and manage consent before attempting release.',
    'An unsafe condition terminated the run unconditionally. Stimulation or determinism fault.'
  ];

  setText('result-title', TITLES[grade] || 'RUN ENDED');
  const titleEl = $('result-title');
  if (titleEl) titleEl.className = CLASSES[grade] || '';
  setText('result-body', BODIES[grade] || REASON_NAME[reason]);

  const statsEl = $('result-stats');
  if (statsEl) {
    const pairs = [
      ['GRADE', GRADE_NAME[grade]], ['STATUS', STATUS_NAME[status]], ['REASON', REASON_NAME[reason]],
      ['SCORE', score.toString()], ['TRUST', trust], ['RISK', risk], ['INTEGRITY', integrity],
      ['EVIDENCE', ['L0','L1','L2','L3'][abi.evidenceLevel()]], ['GATES', gates.toString(2).padStart(7,'0')],
      ['RAW LEAKS', abi.rawLeaks()], ['BEST COMBO', abi.bestCombo()],
      ['MODE', MODE_NAME[mode]], ['SEED', seed.toString(16).padStart(16,'0').toUpperCase()],
      ['TICK', tick], ['HASH', abi.stateHashHex()],
    ];
    statsEl.innerHTML = pairs.map(([k,v]) => `<dt>${k}</dt><dd>${v}</dd>`).join('');
  }

  const verifyEl = $('result-verify');
  if (verifyEl) {
    const modeStr = MODE_NAME[mode]?.toLowerCase() || 'standard';
    if (mode === 5 && date) {
      verifyEl.textContent = `Verify offline: neural-boundary-cli record --mode DAILY --date ${date} --difficulty ${difficulty} --policy clean → verify-all`;
    } else {
      verifyEl.textContent = `Verify offline: neural-boundary-cli record --mode ${modeStr.toUpperCase()} --seed ${seed.toString(16).padStart(16,'0')} --difficulty ${difficulty} → verify-all`;
    }
  }
}

export function fillBlocked() {
  const mask = abi.blockerMask();
  const list = $('blocked-list');
  if (!list) return;
  let html = '';
  for (let g = 0; g < 7; g++) {
    if ((mask >> g) & 1) html += `<li>▢ ${GATE_NAMES[g]} gate not satisfied</li>`;
  }
  if (!(abi.consentScope() & 0x02)) html += `<li>▢ RELEASE scope not in active consent</li>`;
  list.innerHTML = html || '<li>Gates pass but consent is missing — gate a ConsentGrant first</li>';
}
