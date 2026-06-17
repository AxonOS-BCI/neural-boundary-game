// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
// Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
//
// HUD binding (§8). Pure presentation over engine getters.

const RISK_METRICS = new Set(["raw_leak_risk", "stimulation_risk", "latency_pressure"]);
const METRIC_LABELS = {
  boundary_integrity: "Boundary Integrity",
  consent_coherence: "Consent Coherence",
  vault_integrity: "Vault Integrity",
  cognitive_flow: "Cognitive Flow",
  raw_leak_risk: "Raw Leak Risk",
  stimulation_risk: "Stimulation Risk",
  latency_pressure: "Latency Pressure",
  audit_confidence: "Audit Confidence",
};
const RECOMMEND = {
  BENIGN_FLOW: "Authorize — safe flow.",
  CONSENT_REQUEST: "Authorize to keep consent coherent.",
  PERMISSION_ESCALATION: "Audit first, then Revoke if unjustified.",
  RAW_SIGNAL_EXPOSURE: "Seal Vault or Audit before it leaks.",
  STIMULATION_REQUEST: "Throttle before authorizing.",
  UNSAFE_STIMULATION: "Throttle now — never authorize blind.",
  LATENCY_SPIKE: "Quarantine to relieve latency.",
  ADVERSARIAL_PROBE: "Audit, then Quarantine.",
  VAULT_PRESSURE: "Quarantine or Seal Vault.",
  AUDIT_CHECKPOINT: "Audit to raise confidence.",
};
const BLOCKERS = [
  "Wait — minimum run time not yet elapsed",
  "Resolve the active critical event",
  "Boundary Integrity must rise",
  "Consent Coherence must be \u2265 65",
  "Vault Integrity must be \u2265 65",
  "Raw Leak Risk must be \u2264 35",
  "Stimulation Risk must be \u2264 35",
  "Latency Pressure must come down",
];

function band(name, v) {
  const risk = RISK_METRICS.has(name);
  const good = risk ? 100 - v : v;
  if (good >= 65) return "ok";
  if (good >= 40) return "warn";
  return "danger";
}

function scopeText(bits) {
  const parts = [];
  if (bits & 1) parts.push("flow");
  if (bits & 2) parts.push("stim");
  if (bits & 4) parts.push("raw");
  if (bits & 8) parts.push("admin");
  return parts.length ? parts.join("+") : "none";
}

export class Hud {
  constructor(doc) {
    this.d = doc;
    this.metricsEl = doc.getElementById("metrics");
    this._builtMetrics = false;
  }

  topbar(engine) {
    this.d.getElementById("meta-scenario").textContent =
      engine.scenarioId() + " · " + engine.scenarioName(engine.scenarioId());
    this.d.getElementById("meta-seed").textContent = engine.seedHex();
    this.d.getElementById("meta-hash").textContent = engine.hashHex();
    this.d.getElementById("mission-objective").textContent =
      engine.scenarioObjective(engine.scenarioId());
  }

  _buildMetrics() {
    this.metricsEl.innerHTML = "";
    for (const key of Object.keys(METRIC_LABELS)) {
      const el = document.createElement("div");
      el.className = "metric";
      el.dataset.key = key;
      el.dataset.risk = RISK_METRICS.has(key) ? "true" : "false";
      el.innerHTML =
        '<div class="metric-row"><span class="metric-name"></span><span class="metric-val mono"></span></div>' +
        '<div class="bar"><i></i></div>';
      el.querySelector(".metric-name").textContent = METRIC_LABELS[key];
      this.metricsEl.appendChild(el);
    }
    this._builtMetrics = true;
  }

  metrics(m) {
    if (!this._builtMetrics) this._buildMetrics();
    for (const el of this.metricsEl.children) {
      const key = el.dataset.key;
      const v = m[key];
      el.dataset.band = band(key, v);
      el.querySelector(".metric-val").textContent = v;
      el.querySelector(".bar > i").style.width = v + "%";
    }
  }

  eventCard(ev, tick) {
    const card = this.d.getElementById("event-card");
    if (!ev) {
      card.className = "event-card";
      card.innerHTML = '<p class="event-empty">No active event. Watch the field and stabilise.</p>';
      return;
    }
    const hazard = [2, 3, 5, 7].includes(ev.kindCode);
    card.className = "event-card active" + (ev.kindCode === 5 ? " unsafe" : hazard ? " hazard" : "");
    const secs = Math.max(0, (ev.expiresAt - tick) / 20).toFixed(1);
    const auditTxt = ev.requiresAudit
      ? (ev.audited ? '<span class="event-audit-ok">complete</span>' : '<span class="event-audit-bad">incomplete</span>')
      : "n/a";
    card.innerHTML =
      '<p class="event-kind">' + ev.kind.replace(/_/g, " ") + "</p>" +
      '<div class="event-row"><span>Severity</span><b>' + ev.severity + "</b></div>" +
      '<div class="event-row"><span>Visible risk</span><b>' + ev.perceivedRisk + "</b></div>" +
      '<div class="event-row"><span>Scope</span><b>' + scopeText(ev.scope) + "</b></div>" +
      '<div class="event-row"><span>Audit</span><b>' + auditTxt + "</b></div>" +
      '<div class="event-row"><span>Expires in</span><b>' + secs + "s</b></div>" +
      '<p class="event-rec">Recommended: ' + (RECOMMEND[ev.kind] || "Assess and respond.") + "</p>";
  }

  defenses(d) {
    const ul = this.d.getElementById("defenses");
    ul.innerHTML =
      '<li><span>Privacy Vault</span><b>' + (d.vaultSealed ? "sealed" : "open") + " (" + d.vaultCapacity + " left)</b></li>" +
      '<li><span>Permissions</span><b>' + scopeText(d.scopes) + (d.sensitive ? " \u26a0" : "") + "</b></li>" +
      '<li><span>Stimulation</span><b>' + (d.throttled ? "throttled" : "open") + "</b></li>";
  }

  release(rel, terminal) {
    const btn = this.d.getElementById("btn-release");
    const why = this.d.getElementById("release-why");
    if (terminal) {
      btn.dataset.state = "complete";
      btn.textContent = "Run Sealed";
      btn.disabled = true;
      why.textContent = "";
      return;
    }
    if (rel.available) {
      btn.dataset.state = "available";
      btn.textContent = "Release Sovereignty";
      btn.disabled = false;
      why.textContent = "";
    } else {
      const blockers = [];
      for (let i = 0; i < BLOCKERS.length; i++) {
        if (rel.blockers & (1 << i)) blockers.push("\u2022 " + BLOCKERS[i]);
      }
      const nearly = blockers.length <= 2;
      btn.dataset.state = nearly ? "nearly" : "locked";
      btn.textContent = nearly ? "Stabilize Before Release" : "Release Locked";
      btn.disabled = true;
      why.textContent = blockers.length ? "Release locked:\n" + blockers.join("\n") : "";
    }
  }

  status(engine) {
    const pill = this.d.getElementById("status-pill");
    const grade = engine.grade();
    if (engine.terminal()) {
      pill.dataset.grade = grade;
      pill.textContent = engine.gradePublicLabel();
    } else {
      pill.dataset.grade = "";
      pill.textContent = "Running";
    }
    this.d.getElementById("tick-readout").textContent = "t=" + engine.tickCount();
  }

  missionHint(ev) {
    const hint = this.d.getElementById("mission-hint");
    hint.textContent = ev ? (RECOMMEND[ev.kind] || "Assess the active event.") : "Stable. Stabilise further or release when unlocked.";
  }

  lastAction(result) {
    const el = this.d.getElementById("last-action");
    const MAP = {
      ACCEPTED: "", NO_OP: "",
      REJECTED_COOLDOWN: "Too fast — one action per tick.",
      REJECTED_TERMINAL_STATE: "Run has ended.",
      REJECTED_INVALID_FOR_EVENT: "No valid target for that action.",
      REJECTED_RELEASE_LOCKED: "Release is still locked.",
    };
    el.textContent = MAP[result] || "";
  }
}
