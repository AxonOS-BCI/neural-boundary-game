// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Fail-closed WASM fallback (§15). Classic script so it runs even if ES
// modules fail. Never fakes the game and never claims native binaries.
(function () {
  "use strict";
  var MESSAGES = {
    unsupported:
      "Your browser cannot run the deterministic Rust/WASM core required for " +
      "this release. Use a modern browser, or build the CLI from source.",
    offline:
      "Neural Boundary Engine is unavailable offline until the first online " +
      "load completes.",
    mismatch:
      "The cached engine is from a different build and was rejected. Reload " +
      "while online to fetch the matching WASM core.",
    error:
      "The deterministic engine failed to start. Reload while online; if it " +
      "persists, build the CLI from source."
  };

  function show(kind) {
    var el = document.getElementById("fallback");
    var game = document.getElementById("game");
    var menu = document.getElementById("menu");
    if (game) game.hidden = true;
    if (menu) menu.hidden = true;
    if (!el) {
      // Last resort: replace the body.
      document.body.textContent =
        "Neural Boundary Engine requires WebAssembly. " + MESSAGES.unsupported;
      return;
    }
    var msg = MESSAGES[kind] || MESSAGES.error;
    el.innerHTML =
      '<h2>Neural Boundary Engine requires WebAssembly</h2>' +
      "<p>" + msg + "</p>" +
      '<p>The conformance core also runs headless: ' +
      "<code>cargo run -p neural-boundary-cli -- verify-all</code>.</p>";
    el.hidden = false;
  }

  function wasmSupported() {
    try {
      if (typeof WebAssembly !== "object") return false;
      if (typeof WebAssembly.instantiate !== "function") return false;
      var m = new WebAssembly.Module(
        new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])
      );
      return m instanceof WebAssembly.Module;
    } catch (e) {
      return false;
    }
  }

  window.NBG_FALLBACK = { show: show, wasmSupported: wasmSupported };

  if (!wasmSupported()) {
    if (document.readyState !== "loading") show("unsupported");
    else document.addEventListener("DOMContentLoaded", function () { show("unsupported"); });
  }
})();
