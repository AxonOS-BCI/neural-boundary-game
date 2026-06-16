/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 */
// render.js — canvas presentation (§7.1 geometry)

import { abi, KIND_SYMBOL, KIND_COLOR, GATE } from './abi.js';

// Logical field dimensions (§7.1)
const LW = 1024, LH = 576, LANE_Y = [96,192,288,384,480];
const ZONE = 40; // top zone strip
const BG = '#030507', MONO = 'ui-monospace,SFMono-Regular,Menlo,Consolas,monospace';
const CYAN = '#79def5', GOLD = '#d6b96b', SAFE = '#78e6ad', DANGER = '#ff7186';
const LINE = 'rgba(255,255,255,0.10)';

let dpr = 1;
export function setDpr(d) { dpr = d; }

export function fit(canvas) {
  const rect = canvas.getBoundingClientRect();
  const w = Math.max(rect.width, 1), h = Math.max(rect.height, 1);
  const bw = Math.round(w * dpr), bh = Math.round(h * dpr);
  if (canvas.width !== bw) canvas.width = bw;
  if (canvas.height !== bh) canvas.height = bh;
  return [w, h];
}

function metrics(cw, ch) {
  const scale = Math.max(0.05, Math.min(cw / LW, ch / LH));
  return { scale, ox: (cw - LW * scale) / 2, oy: (ch - LH * scale) / 2 };
}

export function laneFromY(canvas, clientY) {
  const rect = canvas.getBoundingClientRect();
  const [cw, ch] = [rect.width, rect.height];
  const { scale, oy } = metrics(cw, ch);
  const ly = (clientY - rect.top - oy) / scale;
  if (ly < ZONE || ly > LH) return -1;
  return Math.min(Math.floor((ly - ZONE) / ((LH - ZONE) / 5)), 4);
}

export function draw(canvas, ctx, reducedMotion) {
  const [cw, ch] = fit(canvas);
  const { scale, ox, oy } = metrics(cw, ch);
  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.fillStyle = BG; ctx.fillRect(0, 0, cw, ch);
  ctx.translate(ox, oy); ctx.scale(scale, scale);

  const bx = 704; // BOUNDARY_X logical
  const aw = 160; // action window width (544..703)
  const laneH = (LH - ZONE) / 5;
  const selLane = abi.selectedLane();

  // Zone tints
  ctx.fillStyle = 'rgba(121,222,245,0.03)';
  ctx.fillRect(bx - aw, ZONE, aw, LH - ZONE);
  ctx.fillStyle = 'rgba(120,230,173,0.025)';
  ctx.fillRect(bx, ZONE, LW - bx, LH - ZONE);

  // Zone labels
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  ctx.font = `600 12px ${MONO}`;
  ctx.fillStyle = 'rgba(255,255,255,0.28)';
  ctx.fillText('SIGNAL', bx * 0.4, ZONE / 2);
  ctx.fillStyle = CYAN;
  ctx.fillText('BOUNDARY', bx, ZONE / 2);
  ctx.fillStyle = SAFE;
  ctx.fillText('APPLICATION', bx + (LW - bx) / 2, ZONE / 2);

  // Lanes
  for (let i = 0; i < 5; i++) {
    const y = ZONE + i * laneH;
    ctx.strokeStyle = LINE; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(LW, y); ctx.stroke();
    if (i === selLane) {
      ctx.fillStyle = 'rgba(121,222,245,0.06)';
      ctx.fillRect(0, y, LW, laneH);
      ctx.fillStyle = CYAN;
      ctx.fillRect(0, y + 4, 4, laneH - 8);
    }
  }

  // Membrane
  ctx.strokeStyle = CYAN; ctx.lineWidth = 3;
  ctx.beginPath(); ctx.moveTo(bx - 2, ZONE); ctx.lineTo(bx - 2, LH); ctx.stroke();
  ctx.strokeStyle = GOLD; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(bx + 4, ZONE); ctx.lineTo(bx + 4, LH); ctx.stroke();

  // Gate mask indicator dots (7 gates along membrane)
  const gateMask = abi.gateMask();
  for (let g = 0; g < 7; g++) {
    const gy = ZONE + (LH - ZONE) * (g + 1) / 8;
    ctx.fillStyle = (gateMask >> g) & 1 ? SAFE : DANGER;
    ctx.beginPath(); ctx.arc(bx - 2, gy, 4, 0, Math.PI * 2); ctx.fill();
  }

  // Entities
  const cap = abi.entityCapacity();
  const r = 17;
  for (let slot = 0; slot < cap; slot++) {
    const kind = abi.entityKind(slot);
    if (kind === 0xFFFFFFFF || kind === 0) continue;
    const lane = abi.entityLane(slot);
    const xQ8 = abi.entityX(slot);
    if (xQ8 === 0xFFFFFFFF) continue;
    const lx = (xQ8 / 256) * LW / LW; // already in logical 0..1024
    const ex = xQ8 / 256;
    const ey = ZONE + lane * laneH + laneH / 2;
    const color = KIND_COLOR[kind] || CYAN;
    const sym = KIND_SYMBOL[kind] || '?';

    // Claim speed trail
    if (!reducedMotion && (kind === 12 || kind === 13 || kind === 14)) {
      ctx.strokeStyle = 'rgba(214,185,107,0.25)';
      ctx.lineWidth = 2;
      ctx.beginPath(); ctx.moveTo(ex - r - 18, ey); ctx.lineTo(ex - r - 4, ey); ctx.stroke();
    }

    // Fill
    ctx.fillStyle = color.replace('#', 'rgba(') + '18)';
    ctx.beginPath(); ctx.arc(ex, ey, r, 0, Math.PI * 2); ctx.fill();

    // Ring
    ctx.strokeStyle = color; ctx.lineWidth = 2.5;
    ctx.beginPath(); ctx.arc(ex, ey, r, 0, Math.PI * 2); ctx.stroke();

    // Validated: double ring
    if (kind === 5) {
      ctx.beginPath(); ctx.arc(ex, ey, r - 5, 0, Math.PI * 2); ctx.stroke();
    }
    // Unknown: dashed ring
    if (kind === 3) {
      const dash = [5, 4];
      ctx.setLineDash(dash);
      ctx.beginPath(); ctx.arc(ex, ey, r, 0, Math.PI * 2); ctx.stroke();
      ctx.setLineDash([]);
    }

    // Symbol
    ctx.fillStyle = kind === 6 ? '#062028' : color;
    ctx.font = `700 15px ${MONO}`;
    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText(sym, ex, ey + 1);

    // Label
    if (scale >= 0.52) {
      ctx.fillStyle = 'rgba(244,241,232,0.42)';
      ctx.font = `500 10px ${MONO}`;
      ctx.fillText(KIND_SYMBOL[kind] ? '' : kind, ex, ey + r + 10);
    }
  }

  ctx.restore();
}
