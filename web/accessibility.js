// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Accessibility (§11) and local preferences (§12). No authoritative gameplay
// state is ever stored.

const NS = "nbg_v7_3_0_";
const ALLOWED = new Set([
  NS + "tutorial_seen", NS + "audio_muted", NS + "reduced_motion",
  NS + "last_scenario", NS + "quality_profile",
]);
const SCHEMA = "nbg-local-preferences-v1";
const PRODUCT = "7.3.0";

function safeLS() {
  try {
    const k = "__nbg_probe__";
    localStorage.setItem(k, "1");
    localStorage.removeItem(k);
    return localStorage;
  } catch (_e) {
    return null;
  }
}

export const Prefs = {
  ls: safeLS(),

  // §12.3 boot cleanup: keep only nbg_v7_3_0_*, drop other nbg_ keys.
  cleanup() {
    if (!this.ls) return;
    const drop = [];
    for (let i = 0; i < this.ls.length; i++) {
      const key = this.ls.key(i);
      if (key && key.startsWith("nbg_") && !key.startsWith(NS)) drop.push(key);
    }
    drop.forEach((k) => this.ls.removeItem(k));
  },

  get(key, fallback) {
    if (!this.ls || !ALLOWED.has(key)) return fallback;
    try {
      const raw = this.ls.getItem(key);
      if (!raw) return fallback;
      const obj = JSON.parse(raw);
      if (obj && obj.schema === SCHEMA) return obj.value;
      return fallback;
    } catch (_e) {
      return fallback; // §12.3 ignore malformed JSON safely
    }
  },

  set(key, value) {
    if (!this.ls || !ALLOWED.has(key)) return;
    const now = new Date().toISOString();
    const obj = {
      schema: SCHEMA, product_version: PRODUCT,
      created_at: now, updated_at: now, ttl_days: 365, value,
    };
    try { this.ls.setItem(key, JSON.stringify(obj)); } catch (_e) { /* quota */ }
  },

  key: (suffix) => NS + suffix,
};

let liveRegion = null;
export function initLiveRegion(doc) {
  liveRegion = doc.createElement("div");
  liveRegion.setAttribute("aria-live", "polite");
  liveRegion.setAttribute("aria-atomic", "true");
  liveRegion.style.cssText =
    "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;";
  doc.body.appendChild(liveRegion);
}
export function announce(msg) {
  if (liveRegion) {
    liveRegion.textContent = "";
    // force re-announcement
    window.requestAnimationFrame(() => { liveRegion.textContent = msg; });
  }
}

export function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function detectQuality(reducedMotion) {
  const cores = navigator.hardwareConcurrency || 2;
  if (cores <= 2 || reducedMotion) return "low";
  if (cores >= 6) return "high";
  return "standard";
}
