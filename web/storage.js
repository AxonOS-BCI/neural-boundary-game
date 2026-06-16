/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 */
// storage.js — local-only persistence under axonos_nbg_v5512_ namespace (§29)

const NS = 'axonos_nbg_v5512_';
const get = k => { try { return localStorage.getItem(NS + k); } catch { return null; } };
const set = (k, v) => { try { localStorage.setItem(NS + k, v); } catch {} };
const del = k => { try { localStorage.removeItem(NS + k); } catch {} };

export function loadPrefs() {
  const raw = get('settings');
  if (!raw) return { mode: 2, difficulty: 1 }; // Standard, Standard
  try { return { ...{ mode: 2, difficulty: 1 }, ...JSON.parse(raw) }; } catch { return { mode: 2, difficulty: 1 }; }
}
export function savePrefs(mode, difficulty) { set('settings', JSON.stringify({ mode, difficulty })); }
export function loadBest(mode) {
  try { const v = get('best_' + mode); return v ? JSON.parse(v) : null; } catch { return null; }
}
export function saveBest(mode, score, grade) {
  const cur = loadBest(mode);
  if (!cur || score > (cur.score || 0)) set('best_' + mode, JSON.stringify({ score, grade }));
}
export function saveLastReplay(data) {
  try { set('last_replay', JSON.stringify(data)); } catch {}
}
export function loadLastReplay() {
  try { const v = get('last_replay'); return v ? JSON.parse(v) : null; } catch { return null; }
}
export function resetAll() {
  try {
    const keys = Object.keys(localStorage).filter(k => k.startsWith(NS));
    keys.forEach(k => localStorage.removeItem(k));
  } catch {}
}
