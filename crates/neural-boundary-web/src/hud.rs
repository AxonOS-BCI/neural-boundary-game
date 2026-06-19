//! DOM mirror: metrics, feedback grammar, target readout, overlays content.

use neural_boundary_core::{
    grand_phase_name, Action, ConsentState, Difficulty, Event, EvidenceLevel, ReleaseBlocker,
    RunMode, Simulation, Snapshot, Status, TerminalReason, BOUNDARY_X, GATE_WINDOW, REVIEW_GATES,
    TICKS_PER_SECOND,
};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Bad,
}

impl Tone {
    const fn class(self) -> &'static str {
        match self {
            Self::Ok => "feedback ok",
            Self::Warn => "feedback warn",
            Self::Bad => "feedback bad",
        }
    }
}

pub struct Ui {
    document: Document,
    pub app_root: HtmlElement,
    pub canvas: web_sys::HtmlCanvasElement,
    game_sections: Vec<HtmlElement>,
    m_trust: Element,
    m_risk: Element,
    m_integrity: Element,
    m_evidence: Element,
    m_consent: Element,
    m_gates: Element,
    m_score: Element,
    m_streak: Element,
    b_trust: HtmlElement,
    b_risk: HtmlElement,
    b_integrity: HtmlElement,
    hud_mode: Element,
    hud_phase: HtmlElement,
    seed_chip: Element,
    feedback: Element,
    target: Element,
    btn_pause: Element,
    act_buttons: Vec<Element>,
    pub ov_help: HtmlElement,
    pub ov_pause: HtmlElement,
    pub ov_blocked: HtmlElement,
    pub ov_result: HtmlElement,
    pub ov_reset: HtmlElement,
    blocked_list: Element,
    result_title: Element,
    result_body: Element,
    result_stats: Element,
    result_verify: Element,
    mode_blurb: Element,
}

fn el(document: &Document, id: &str) -> Element {
    document
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing #{id}"))
}

fn hel(document: &Document, id: &str) -> HtmlElement {
    el(document, id).dyn_into().expect("HtmlElement")
}

impl Ui {
    pub fn new(document: &Document) -> Self {
        let game_sections = ["game-header", "hud", "field-wrap", "deck", "foot"]
            .iter()
            .map(|id| hel(document, id))
            .collect();
        let act_buttons = ["1", "2", "3", "4", "5", "release"]
            .iter()
            .map(|key| {
                document
                    .query_selector(&format!("[data-act='{key}']"))
                    .ok()
                    .flatten()
                    .expect("action button")
            })
            .collect();
        let ui = Self {
            app_root: hel(document, "app"),
            canvas: el(document, "field").dyn_into().expect("canvas"),
            game_sections,
            m_trust: el(document, "m-trust"),
            m_risk: el(document, "m-risk"),
            m_integrity: el(document, "m-integrity"),
            m_evidence: el(document, "m-evidence"),
            m_consent: el(document, "m-consent"),
            m_gates: el(document, "m-gates"),
            m_score: el(document, "m-score"),
            m_streak: el(document, "m-streak"),
            b_trust: hel(document, "b-trust"),
            b_risk: hel(document, "b-risk"),
            b_integrity: hel(document, "b-integrity"),
            hud_mode: el(document, "hud-mode"),
            hud_phase: hel(document, "hud-phase"),
            seed_chip: el(document, "seed-chip"),
            feedback: el(document, "feedback"),
            target: el(document, "target-readout"),
            btn_pause: el(document, "btn-pause"),
            act_buttons,
            ov_help: hel(document, "ov-help"),
            ov_pause: hel(document, "ov-pause"),
            ov_blocked: hel(document, "ov-blocked"),
            ov_result: hel(document, "ov-result"),
            ov_reset: hel(document, "ov-reset"),
            blocked_list: el(document, "blocked-list"),
            result_title: el(document, "result-title"),
            result_body: el(document, "result-body"),
            result_stats: el(document, "result-stats"),
            result_verify: el(document, "result-verify"),
            mode_blurb: el(document, "mode-blurb"),
            document: document.clone(),
        };
        ui.fill_legend();
        ui
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn reveal_game(&self) {
        for section in &self.game_sections {
            let _ = section.remove_attribute("hidden");
        }
        let _ = self.app_root.set_attribute("data-phase", "game");
    }

    pub fn show_landing(&self) {
        let _ = self.app_root.set_attribute("data-phase", "landing");
    }

    pub fn set_pause_label(&self, paused: bool) {
        self.btn_pause
            .set_text_content(Some(if paused { "RESUME" } else { "PAUSE" }));
    }

    pub fn set_mode_chip(&self, mode: RunMode, difficulty: Difficulty) {
        self.hud_mode.set_text_content(Some(&format!(
            "{} · {}",
            mode.name().to_uppercase(),
            difficulty.name().to_uppercase()
        )));
    }

    pub fn set_seed(&self, seed: u64, mode: RunMode, date: Option<&str>) {
        let text = match (mode, date) {
            (RunMode::Daily, Some(date)) => format!("DAILY {date} · SEED 0x{seed:016X}"),
            _ => format!("SEED 0x{seed:016X}"),
        };
        self.seed_chip.set_text_content(Some(&text));
    }

    pub fn set_feedback(&self, text: &str, tone: Tone) {
        self.feedback.set_text_content(Some(text));
        let _ = self.feedback.set_attribute("class", tone.class());
    }

    pub fn update(&self, simulation: &Simulation) {
        let snapshot = simulation.snapshot();
        self.m_trust
            .set_text_content(Some(&snapshot.trust.to_string()));
        self.m_risk
            .set_text_content(Some(&snapshot.risk.to_string()));
        self.m_integrity
            .set_text_content(Some(&snapshot.integrity.to_string()));
        self.m_evidence.set_text_content(Some(&format!(
            "{} · {}",
            snapshot.evidence_level.as_str(),
            snapshot.evidence_points
        )));
        self.m_gates
            .set_text_content(Some(&format!("{}/{}", snapshot.gates_passed, REVIEW_GATES)));
        self.m_score
            .set_text_content(Some(&snapshot.score.to_string()));
        self.m_streak
            .set_text_content(Some(&snapshot.streak.to_string()));

        let set_bar = |bar: &HtmlElement, value: i32| {
            let _ = bar
                .style()
                .set_property("width", &format!("{}%", value.clamp(0, 100)));
        };
        set_bar(&self.b_trust, snapshot.trust);
        set_bar(&self.b_risk, snapshot.risk);
        set_bar(&self.b_integrity, snapshot.integrity);

        let (consent_text, consent_class) = match snapshot.consent {
            ConsentState::Inactive => ("INACTIVE".to_string(), "v"),
            ConsentState::Active { .. } => (
                format!(
                    "ACTIVE · {}s",
                    snapshot.consent_remaining.div_ceil(TICKS_PER_SECOND)
                ),
                "v on",
            ),
            ConsentState::Revoked { .. } => ("REVOKED ⛔".to_string(), "v revoked"),
        };
        self.m_consent.set_text_content(Some(&consent_text));
        let _ = self.m_consent.set_attribute("class", consent_class);
        let _ = self.m_consent.set_attribute("id", "m-consent");

        if snapshot.mode == RunMode::Grand {
            let _ = self.hud_phase.remove_attribute("hidden");
            self.hud_phase.set_text_content(Some(&format!(
                "PHASE {} · {}",
                snapshot.phase + 1,
                grand_phase_name(snapshot.phase).to_uppercase()
            )));
        } else {
            let _ = self.hud_phase.set_attribute("hidden", "");
        }

        for button in &self.act_buttons {
            let _ = button
                .class_list()
                .toggle_with_force("cooling", snapshot.cooldown > 0);
        }

        self.update_target(simulation, &snapshot);
    }

    fn update_target(&self, simulation: &Simulation, snapshot: &Snapshot) {
        let mut frontmost: Option<&neural_boundary_core::Entity> = None;
        for entity in simulation.entities().iter().flatten() {
            if entity.lane != snapshot.selected_lane
                || entity.x < BOUNDARY_X - GATE_WINDOW
                || entity.x >= BOUNDARY_X
            {
                continue;
            }
            if frontmost.map(|front| entity.x > front.x).unwrap_or(true) {
                frontmost = Some(entity);
            }
        }
        let text = match frontmost {
            Some(entity) => {
                let hint = entity
                    .kind
                    .required_action()
                    .map(action_hint)
                    .unwrap_or("crossing is legal");
                format!(
                    "LANE {} ▸ {} {} — {}",
                    snapshot.selected_lane + 1,
                    entity.kind.symbol(),
                    entity.kind.label(),
                    hint
                )
            }
            None => format!(
                "LANE {} ▸ no target in the gate window",
                snapshot.selected_lane + 1
            ),
        };
        self.target.set_text_content(Some(&text));
    }

    fn fill_legend(&self) {
        let Some(list) = self.document.get_element_by_id("help-legend") else {
            return;
        };
        let mut html = String::new();
        for kind in neural_boundary_core::EntityKind::ALL {
            html.push_str(&format!(
                "<li><span class=\"sym\">{}</span><span><b>{}</b> — {}</span></li>",
                kind.symbol(),
                kind.label(),
                kind.description()
            ));
        }
        list.set_inner_html(&html);
    }

    pub fn fill_blocked(&self, blockers: &[ReleaseBlocker]) {
        let mut html = String::new();
        for blocker in blockers {
            html.push_str(&format!("<li>▢ {}</li>", blocker.as_str()));
        }
        self.blocked_list.set_inner_html(&html);
    }

    pub fn fill_result(&self, simulation: &Simulation, seed: u64, date: Option<&str>) {
        let snapshot = simulation.snapshot();
        let reason = match snapshot.status {
            Status::Terminal(reason) => reason,
            Status::Running => TerminalReason::HorizonReached,
        };
        let (title, class, body) = result_copy(reason);
        self.result_title.set_text_content(Some(title));
        let _ = self.result_title.set_attribute("class", class);
        let _ = self.result_title.set_attribute("id", "result-title");
        self.result_body.set_text_content(Some(body));

        let weakest = simulation
            .weakest_control()
            .map(|bucket| bucket.name())
            .unwrap_or("NONE");
        let mode = snapshot.mode;
        let pairs: Vec<(&str, String)> = vec![
            ("GRADE", simulation.grade().name().to_string()),
            ("REASON", reason.schema_str().to_string()),
            ("SCORE", snapshot.score.to_string()),
            ("TRUST", snapshot.trust.to_string()),
            ("RISK", snapshot.risk.to_string()),
            ("INTEGRITY", snapshot.integrity.to_string()),
            (
                "EVIDENCE",
                format!(
                    "{} · {} pts",
                    snapshot.evidence_level.as_str(),
                    snapshot.evidence_points
                ),
            ),
            (
                "GATES",
                format!("{}/{}", snapshot.gates_passed, REVIEW_GATES),
            ),
            ("RAW LEAKS", snapshot.raw_leaks.to_string()),
            ("BEST STREAK", snapshot.best_streak.to_string()),
            ("WEAKEST CONTROL", weakest.to_string()),
            (
                "MODE",
                format!("{} · {}", mode.name(), snapshot.difficulty.name()),
            ),
            ("SEED", format!("0x{seed:016x}")),
            ("TERMINAL TICK", snapshot.tick.to_string()),
            ("STATE HASH", format!("0x{:016x}", simulation.state_hash())),
        ];
        let mut html = String::new();
        for (key, value) in pairs {
            html.push_str(&format!("<dt>{key}</dt><dd>{value}</dd>"));
        }
        self.result_stats.set_inner_html(&html);

        let verify = match (mode, date) {
            (RunMode::Daily, Some(date)) => format!(
                "Pin this exact run offline: neural-boundary-cli record --mode daily --date {date} --policy clean — then verify-all."
            ),
            _ => format!(
                "Pin this world offline: neural-boundary-cli record --mode {} --difficulty {} --seed {seed} — then verify-all.",
                mode.name(),
                snapshot.difficulty.name()
            ),
        };
        self.result_verify.set_text_content(Some(&verify));
    }

    pub fn set_mode_blurb(&self, mode: RunMode) {
        self.mode_blurb.set_text_content(Some(mode_blurb(mode)));
    }
}

const fn action_hint(action: Action) -> &'static str {
    match action {
        Action::Validate => "press 1 VALIDATE",
        Action::Convert => "press 2 CONVERT",
        Action::Quarantine => "press 3 QUARANTINE",
        Action::ConsentGate => "press 4 CONSENT",
        Action::EvidenceGate => "press 5 EVIDENCE",
        Action::Release => "press ⏎ RELEASE",
    }
}

pub const fn mode_blurb(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Guided => "A 60-second teaching run. The boundary coaches every action once: contain, validate, prove, consent, convert, seal.",
        RunMode::Standard => "Canonical balanced run. Seal the boundary: trust 90, risk 20, integrity 80, evidence L2, five gates, zero leaks.",
        RunMode::Audit => "Stricter review: evidence L3, risk ceiling 15, more deceptive packets and revoked credentials.",
        RunMode::Grand => "Four phases — Signal Integrity, Consent and Evidence, Release Under Pressure, Sovereign Boundary Review. Release opens only in the final phase.",
        RunMode::Daily => "One deterministic world per UTC date, derived with a documented algorithm. Same date, same seed — everywhere, no backend.",
    }
}

pub const fn guided_hint(hint: u8) -> &'static str {
    match hint {
        0 => "COACH · Select the lane (▲▼ or tap), then press 3 — QUARANTINE the RAW frame.",
        1 => "COACH · An INTENT candidate approaches. Press 1 — VALIDATE it.",
        2 => "COACH · Proof incoming. Press 5 — register EVIDENCE.",
        3 => "COACH · A CONSENT token. Press 4 — open the consent gate.",
        4 => "COACH · Consent and evidence are ready. VALIDATE (1), then 2 — CONVERT to typed.",
        5 => "COACH · Claims move faster than evidence. Press 3 — QUARANTINE before they cross.",
        _ => "COACH · When every review gate passes, press ⏎ — RELEASE seals the boundary.",
    }
}

fn result_copy(reason: TerminalReason) -> (&'static str, &'static str, &'static str) {
    match reason {
        TerminalReason::Sealed => (
            "BOUNDARY SEALED",
            "sealed",
            "Raw signal stayed private. Applications received typed intent only.",
        ),
        TerminalReason::HorizonReached => (
            "REVIEW WINDOW CLOSED",
            "expired",
            "The run reached its horizon without a seal. The boundary held, but the release was never completed.",
        ),
        TerminalReason::IntegrityCollapse => (
            "INTEGRITY COLLAPSE",
            "breached",
            "Unsupported claims and artifacts eroded the boundary until nothing it reported could be trusted.",
        ),
        TerminalReason::RiskOverflow => (
            "RISK OVERFLOW",
            "breached",
            "Accumulated review risk exceeded the budget. The boundary can no longer be defended.",
        ),
        TerminalReason::RawLeakThreshold => (
            "RAW SIGNAL BREACH",
            "breached",
            "Private raw frames crossed the boundary three times. Fix the containment path and try again.",
        ),
        TerminalReason::StimulationCrossed => (
            "STIMULATION FAIL-CLOSED",
            "breached",
            "A stimulation command reached the membrane. The boundary fails closed — immediately and unconditionally.",
        ),
    }
}

/// Feedback grammar: action → immediate signal → concise reason.
/// Returns `(line, tone, announce)`; `announce` is read by screen readers.
pub fn event_copy(event: Event) -> Option<(String, Tone, Option<String>)> {
    use Event::*;
    Some(match event {
        Validated(count) => (
            format!("INTENT TYPED · schema validation {count}"),
            Tone::Ok,
            None,
        ),
        Revealed(kind) => (
            format!("?PKT CLASSIFIED · {} · risk +4", kind.label()),
            Tone::Warn,
            Some(format!("Packet classified as {}", kind.label())),
        ),
        Converted => (
            "INTENT CONVERTED · typed for delivery".into(),
            Tone::Ok,
            None,
        ),
        ConvertBlockedConsent(state) => {
            let reason = match state {
                ConsentState::Revoked { .. } => "consent was revoked",
                _ => "the consent gate is closed",
            };
            (
                format!("CONVERT BLOCKED · {reason}"),
                Tone::Warn,
                Some(format!("Conversion blocked: {reason}")),
            )
        }
        ConvertBlockedEvidence => (
            "CONVERT BLOCKED · evidence below L1".into(),
            Tone::Warn,
            None,
        ),
        Quarantined(kind) => (
            format!("{} CONTAINED · stayed inside the boundary", kind.label()),
            Tone::Ok,
            None,
        ),
        QuarantineBlockedUnknown => (
            "QUARANTINE BLOCKED · classify the ?PKT first".into(),
            Tone::Warn,
            None,
        ),
        FalsePositive(kind) => (
            format!(
                "FALSE POSITIVE · {} was legitimate · trust −3",
                kind.label()
            ),
            Tone::Bad,
            Some("False positive quarantine".into()),
        ),
        ConsentOn { .. } => (
            "CONSENT OPEN · conversion scope active 25 s".into(),
            Tone::Ok,
            Some("Consent gate open".into()),
        ),
        ConsentExpired => (
            "CONSENT EXPIRED · the gate closed".into(),
            Tone::Warn,
            Some("Consent expired".into()),
        ),
        ConsentRevoked { .. } => (
            "CONSENT REVOKED · the capability is no longer valid".into(),
            Tone::Bad,
            Some("Consent revoked immediately".into()),
        ),
        ConsentRevokedToken => (
            "CONSENT REVOKED · quarantine the stale credential".into(),
            Tone::Warn,
            None,
        ),
        EvidenceUp(level, points) => (
            format!("EVIDENCE REGISTERED · {} ({points} pts)", level.as_str()),
            Tone::Ok,
            (level >= EvidenceLevel::L2).then(|| format!("Evidence level {}", level.as_str())),
        ),
        GatePassed(index) => (
            format!(
                "REVIEW GATE PASSED · {}",
                neural_boundary_core::gate_name(index)
            ),
            Tone::Ok,
            Some(format!(
                "Review gate {} passed",
                neural_boundary_core::gate_name(index)
            )),
        ),
        Delivered => (
            "TYPED INTENT DELIVERED · trust +6".into(),
            Tone::Ok,
            Some("Typed intent delivered".into()),
        ),
        MissedIntent(kind) => (
            format!("{} MISSED · crossed unprocessed · trust −1", kind.label()),
            Tone::Warn,
            None,
        ),
        LostArtifact(kind) => (
            format!("{} LOST · crossed unused", kind.label()),
            Tone::Warn,
            None,
        ),
        MinorBreach(kind) => (
            format!("{} CROSSED · integrity −6", kind.label()),
            Tone::Bad,
            Some(format!("{} crossed the boundary", kind.label())),
        ),
        RawLeakBreach => (
            "RAW LEAK · private signal crossed · integrity −10".into(),
            Tone::Bad,
            Some("Raw signal leak".into()),
        ),
        StimBreach => (
            "STIM CROSSED · boundary fails closed".into(),
            Tone::Bad,
            Some("Stimulation command crossed. Boundary failed closed.".into()),
        ),
        WrongTarget(action, kind) => (
            format!(
                "{} · wrong target ({})",
                action.name().to_uppercase(),
                kind.label()
            ),
            Tone::Warn,
            None,
        ),
        NoTarget(action) => (
            format!(
                "{} · no target in the gate window",
                action.name().to_uppercase()
            ),
            Tone::Warn,
            None,
        ),
        ReleaseRejected(blocker) => (
            format!("RELEASE BLOCKED · {}", blocker.as_str()),
            Tone::Bad,
            Some("Release blocked".into()),
        ),
        ReleaseSealed => (
            "RELEASE ACCEPTED · boundary sealed".into(),
            Tone::Ok,
            Some("Boundary sealed".into()),
        ),
        PhaseChanged(phase) => (
            format!(
                "PHASE {} · {}",
                phase + 1,
                grand_phase_name(phase).to_uppercase()
            ),
            Tone::Warn,
            Some(format!("Phase {}: {}", phase + 1, grand_phase_name(phase))),
        ),
        GuidedStep(hint) => (
            guided_hint(hint).to_string(),
            Tone::Ok,
            Some(guided_hint(hint).to_string()),
        ),
        Terminal(_) => return None,
    })
}
