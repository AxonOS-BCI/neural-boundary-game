// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Boundary Field renderer (§9). Canvas, no engine. Animation is
// presentation-only and never mutates deterministic state (§9.2).

const C = {
  bg0: "#050608", line: "#1c2530", gold: "#C8A96A", platinum: "#D8DEE7",
  safe: "#83D9B5", warn: "#E8C16A", danger: "#E46F6F", unsafe: "#C85CFF",
};

export class BoundaryField {
  constructor(canvas, quality, reducedMotion) {
    this.cv = canvas;
    this.ctx = canvas.getContext("2d");
    this.quality = quality || "standard";
    this.reduced = !!reducedMotion;
    this.t = 0;
  }
  setReducedMotion(v) { this.reduced = v; }

  // snapshot: { metrics, defenses, event, grade, status, release }
  draw(snap) {
    const ctx = this.ctx;
    const W = this.cv.width, H = this.cv.height;
    const cx = W / 2, cy = H / 2;
    const R = Math.min(W, H) * 0.42;
    if (!this.reduced) this.t += 1;
    const breathe = this.reduced ? 0 : Math.sin(this.t * 0.05) * 3;
    const m = snap.metrics;

    ctx.clearRect(0, 0, W, H);

    // 1. Outer threat perimeter — density tracks aggregate risk.
    const risk = (m.raw_leak_risk + m.stimulation_risk + m.latency_pressure) / 300;
    ctx.strokeStyle = "rgba(228,111,111," + (0.12 + risk * 0.5).toFixed(3) + ")";
    ctx.lineWidth = 1;
    const spokes = this.quality === "low" ? 24 : 48;
    for (let i = 0; i < spokes; i++) {
      const a = (i / spokes) * Math.PI * 2;
      const j = this.reduced ? 0 : Math.sin(this.t * 0.04 + i) * 4 * risk;
      ctx.beginPath();
      ctx.moveTo(cx + Math.cos(a) * (R + 14), cy + Math.sin(a) * (R + 14));
      ctx.lineTo(cx + Math.cos(a) * (R + 26 + j), cy + Math.sin(a) * (R + 26 + j));
      ctx.stroke();
    }

    // 2. Boundary integrity ring — color + completeness by boundary metric.
    const bi = m.boundary_integrity / 100;
    const ringColor = bi > 0.7 ? C.safe : bi > 0.4 ? C.warn : C.danger;
    ctx.lineWidth = 6;
    ctx.strokeStyle = ringColor;
    ctx.globalAlpha = 0.9;
    ctx.beginPath();
    ctx.arc(cx, cy, R + breathe, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * bi);
    ctx.stroke();
    // faint full ring for reference
    ctx.globalAlpha = 0.15;
    ctx.beginPath(); ctx.arc(cx, cy, R + breathe, 0, Math.PI * 2); ctx.stroke();
    ctx.globalAlpha = 1;

    // breach fractures
    if (snap.grade === "BREACHED" || snap.grade === "UNSAFE") {
      ctx.strokeStyle = snap.grade === "UNSAFE" ? C.unsafe : C.danger;
      ctx.lineWidth = 2;
      for (let i = 0; i < 6; i++) {
        const a = (i / 6) * Math.PI * 2 + 0.4;
        ctx.beginPath();
        ctx.moveTo(cx + Math.cos(a) * R * 0.4, cy + Math.sin(a) * R * 0.4);
        ctx.lineTo(cx + Math.cos(a) * (R + 24), cy + Math.sin(a) * (R + 24));
        ctx.stroke();
      }
    }

    // 3. Privacy Vault shield
    const d = snap.defenses;
    if (d.vaultSealed) {
      ctx.strokeStyle = C.gold; ctx.lineWidth = 3; ctx.globalAlpha = 0.8;
      ctx.beginPath(); ctx.arc(cx, cy, R * 0.72, 0, Math.PI * 2); ctx.stroke();
      ctx.strokeStyle = C.platinum; ctx.globalAlpha = 0.35;
      ctx.beginPath(); ctx.arc(cx, cy, R * 0.69, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
    } else {
      const vi = d.vaultCapacity;
      ctx.strokeStyle = "rgba(200,169,106,0.25)"; ctx.lineWidth = 1;
      ctx.beginPath(); ctx.arc(cx, cy, R * 0.72, 0, Math.PI * 2); ctx.stroke();
      ctx.fillStyle = C.gold; ctx.font = "11px ui-monospace, monospace";
      ctx.textAlign = "center"; ctx.globalAlpha = 0.6;
      ctx.fillText("vault " + vi, cx, cy - R * 0.72 - 6);
      ctx.globalAlpha = 1;
    }

    // 4. Permission arcs (scope bits: FLOW1 STIM2 RAW4 ADMIN8)
    const scopeColors = [[1, C.safe], [2, C.warn], [4, C.danger], [8, C.unsafe]];
    let arc = -Math.PI / 2;
    scopeColors.forEach(([bit, col]) => {
      if (d.scopes & bit) {
        ctx.strokeStyle = col; ctx.lineWidth = 4; ctx.globalAlpha = 0.85;
        ctx.beginPath(); ctx.arc(cx, cy, R * 0.84, arc, arc + 0.5); ctx.stroke();
        arc += 0.65;
      }
    });
    ctx.globalAlpha = 1;

    // 5. Inner cognition core — flow brightness; consent tints.
    const flow = m.cognitive_flow / 100;
    const consent = m.consent_coherence / 100;
    const coreR = R * 0.34 + (this.reduced ? 0 : Math.sin(this.t * 0.06) * 2);
    const grad = ctx.createRadialGradient(cx, cy, 2, cx, cy, coreR);
    const core = consent > 0.5 ? C.platinum : C.warn;
    grad.addColorStop(0, core);
    grad.addColorStop(1, "rgba(8,12,18," + (1 - flow * 0.5).toFixed(2) + ")");
    ctx.fillStyle = grad; ctx.globalAlpha = 0.55 + flow * 0.4;
    ctx.beginPath(); ctx.arc(cx, cy, coreR, 0, Math.PI * 2); ctx.fill();
    ctx.globalAlpha = 1;

    // 6. Incoming event pulse
    if (snap.event) {
      const ev = snap.event;
      const hazard = ev.kindCode === 3 || ev.kindCode === 5 || ev.kindCode === 2 || ev.kindCode === 7;
      const col = ev.kindCode === 5 ? C.unsafe : hazard ? C.danger : C.safe;
      const pulse = this.reduced ? 0.5 : (Math.sin(this.t * 0.12) + 1) / 2;
      ctx.strokeStyle = col; ctx.globalAlpha = 0.3 + pulse * 0.6; ctx.lineWidth = 2;
      const er = R * (0.45 + 0.4 * (1 - pulse));
      ctx.beginPath(); ctx.arc(cx, cy, er, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      // raw leak filament crossing the boundary
      if (ev.kindCode === 3 && !d.vaultSealed) {
        ctx.strokeStyle = C.danger; ctx.lineWidth = 1.5; ctx.globalAlpha = 0.7;
        const a = (ev.id % 8) / 8 * Math.PI * 2;
        ctx.beginPath();
        ctx.moveTo(cx + Math.cos(a) * (R + 20), cy + Math.sin(a) * (R + 20));
        ctx.lineTo(cx + Math.cos(a) * coreR, cy + Math.sin(a) * coreR);
        ctx.stroke(); ctx.globalAlpha = 1;
      }
    }

    // 7. Sealed release glow
    if (snap.grade === "SEALED") {
      ctx.strokeStyle = C.safe; ctx.globalAlpha = 0.5; ctx.lineWidth = 10;
      ctx.beginPath(); ctx.arc(cx, cy, R + 8, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.fillStyle = C.gold; ctx.font = "600 20px Inter, sans-serif";
      ctx.textAlign = "center"; ctx.fillText("\u2713 SEALED", cx, cy + 5);
    } else if (snap.release && snap.release.available) {
      ctx.strokeStyle = C.gold; ctx.globalAlpha = 0.25 + (this.reduced ? 0.2 : Math.sin(this.t * 0.1) * 0.2 + 0.2);
      ctx.lineWidth = 3;
      ctx.beginPath(); ctx.arc(cx, cy, R + 6, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
    }
  }
}
