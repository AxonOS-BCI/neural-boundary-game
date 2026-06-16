/*
 * SPDX-FileCopyrightText: 2026 Denis Yermakou
 * SPDX-FileContributor: AxonOS
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
 */
// a11y.js — WCAG 2.2 AA helpers: announcements, focus management, trap

export function announce(text) {
  const live = document.getElementById('live-region');
  if (!live) return;
  live.textContent = '';
  requestAnimationFrame(() => { live.textContent = text; });
}

export function focusFirst(container) {
  const el = container.querySelector('button,[href],input,[tabindex]:not([tabindex="-1"])');
  if (el) el.focus();
}

export function trapFocus(container, event) {
  if (event.key !== 'Tab') return;
  const items = Array.from(container.querySelectorAll(
    'button,[href],input,select,[tabindex]:not([tabindex="-1"])'
  )).filter(el => !el.disabled);
  if (!items.length) return;
  const first = items[0], last = items[items.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault(); last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault(); first.focus();
  }
}
