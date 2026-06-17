// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Audio cues (§10). Web Audio API only, no external files, muted by default,
// resumed only after a user gesture. Restrained, non-arcade.

const CUES = {
  AUDIT_ACCEPTED: { f: 880, t: "sine", d: 0.06, g: 0.05 },
  AUTHORIZE_SAFE: { f: 520, t: "sine", d: 0.08, g: 0.06 },
  REVOKE_PERMISSION: { f: 200, t: "square", d: 0.07, g: 0.04 },
  QUARANTINE: { f: 150, t: "triangle", d: 0.12, g: 0.05 },
  SEAL_VAULT: { f: 320, t: "sine", d: 0.18, g: 0.06, sweepTo: 180 },
  THROTTLE: { f: 300, t: "sawtooth", d: 0.16, g: 0.04, sweepTo: 120 },
  RISK_SPIKE: { f: 440, t: "triangle", d: 0.1, g: 0.05 },
  RAW_LEAK_WARNING: { f: 1200, t: "sine", d: 0.14, g: 0.03 },
  UNSAFE_STIMULATION: { f: 700, t: "square", d: 0.1, g: 0.06 },
  BREACH: { f: 80, t: "sawtooth", d: 0.5, g: 0.08 },
  SEALED_RELEASE: { f: 528, t: "sine", d: 0.5, g: 0.06, sweepTo: 660 },
};

export class Audio {
  constructor(muted) {
    this.muted = muted !== false; // default muted
    this.ctx = null;
  }
  _ensure() {
    if (!this.ctx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (AC) this.ctx = new AC();
    }
    if (this.ctx && this.ctx.state === "suspended") this.ctx.resume();
    return this.ctx;
  }
  setMuted(v) { this.muted = v; if (!v) this._ensure(); }
  // Call from a user gesture to unlock audio.
  unlock() { if (!this.muted) this._ensure(); }

  play(cue) {
    if (this.muted) return;
    const ctx = this._ensure();
    if (!ctx) return;
    const c = CUES[cue];
    if (!c) return;
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = c.t;
    osc.frequency.setValueAtTime(c.f, now);
    if (c.sweepTo) osc.frequency.exponentialRampToValueAtTime(c.sweepTo, now + c.d);
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(c.g, now + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + c.d);
    osc.connect(gain).connect(ctx.destination);
    osc.start(now);
    osc.stop(now + c.d + 0.02);
  }
}

// Map engine action/result/grade transitions to cues.
export function cueForAction(actionCode) {
  return ["", "AUDIT_ACCEPTED", "AUTHORIZE_SAFE", "REVOKE_PERMISSION",
    "QUARANTINE", "SEAL_VAULT", "THROTTLE", ""][actionCode] || "";
}
