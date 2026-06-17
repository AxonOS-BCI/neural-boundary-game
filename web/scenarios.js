// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// Builds the scenario menu from engine metadata (§8.2). No hardcoded copy —
// names, objectives, and difficulty come from the deterministic core.

export function buildMenu(engine, listEl, onSelect) {
  listEl.innerHTML = "";
  const count = engine.scenarioCount();
  for (let id = 1; id <= count; id++) {
    const li = document.createElement("li");
    const card = document.createElement("button");
    card.className = "scenario-card";
    card.type = "button";
    const diff = engine.scenarioDifficulty(id);
    card.dataset.diff = diff;
    card.setAttribute("aria-label", "Scenario " + id + ": " + engine.scenarioName(id) + ", " + diff);
    card.innerHTML =
      '<span class="sc-head"><span class="sc-name"></span><span class="sc-diff"></span></span>' +
      '<span class="sc-obj"></span>';
    card.querySelector(".sc-name").textContent = id + ". " + engine.scenarioName(id);
    card.querySelector(".sc-diff").textContent = diff;
    card.querySelector(".sc-obj").textContent = engine.scenarioObjective(id);
    card.addEventListener("click", () => onSelect(id));
    li.appendChild(card);
    listEl.appendChild(li);
  }
}
