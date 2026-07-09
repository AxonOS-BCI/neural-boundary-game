// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
//
// Self-heal: when the build version changes, evict any service worker and
// caches left by a previous deploy, then reload once so the current tree is
// served fresh. This kills the "stale old shell / stuck launch overlay"
// failure mode where an old service worker keeps serving a previous version.
// Runs as a classic (non-module) script in <head>, before the app boots.

(function () {
  "use strict";
  var BUILD = "8.2.1";

  function finish() {
    try { localStorage.setItem("nbg_build", BUILD); } catch (e) { /* ignore */ }
    var healed = false;
    try { healed = sessionStorage.getItem("nbg_healed") === "1"; } catch (e) { /* ignore */ }
    if (!healed) {
      try { sessionStorage.setItem("nbg_healed", "1"); } catch (e) { /* ignore */ }
      location.reload();
    }
  }

  try {
    var prev = null;
    try { prev = localStorage.getItem("nbg_build"); } catch (e) { /* ignore */ }

    if (prev === BUILD) {
      try { sessionStorage.removeItem("nbg_healed"); } catch (e) { /* ignore */ }
      return;
    }

    var tasks = [];
    if ("serviceWorker" in navigator) {
      tasks.push(
        navigator.serviceWorker.getRegistrations()
          .then(function (regs) {
            return Promise.all(regs.map(function (r) { return r.unregister(); }));
          })
          .catch(function () { /* ignore */ })
      );
    }
    if (window.caches && caches.keys) {
      tasks.push(
        caches.keys()
          .then(function (keys) {
            return Promise.all(keys.map(function (k) { return caches.delete(k); }));
          })
          .catch(function () { /* ignore */ })
      );
    }

    if (tasks.length === 0) {
      finish();
    } else {
      Promise.all(tasks).then(finish, finish);
    }
  } catch (e) {
    /* never block the app on self-heal */
  }
})();
