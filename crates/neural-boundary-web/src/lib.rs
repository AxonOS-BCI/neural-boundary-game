//! neural-boundary-web — Rust/WASM front-end for the Neural Boundary Game
//! (v2.1.2, Foundation Grande AxonOS Standard Edition).
//!
//! All simulation logic lives in `neural-boundary-core`. This crate renders
//! the playfield to a canvas, mirrors metrics into the HTML status rail, and
//! translates keyboard/pointer input into deterministic core `Input` ticks.

#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub fn native_placeholder() -> &'static str {
    "neural-boundary-web is intended for wasm32 builds"
}

#[cfg(target_arch = "wasm32")]
mod app {
    use core::cell::RefCell;
    use neural_boundary_core::{
        Action, Difficulty, Entity, EntityKind, Event as CoreEvent, GameConfig, GameState, Input,
        ReleaseBlocker, Status, BOUNDARY_X, FIELD_END_X, GATE_WINDOW, LANES, TICKS_PER_SECOND,
    };
    use std::collections::VecDeque;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{
        CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement, KeyboardEvent,
        MouseEvent, Window,
    };

    // Stage geometry (CSS pixels; the canvas backing store is DPR-scaled).
    const CANVAS_W: f64 = 916.0;
    const CANVAS_H: f64 = 372.0;
    const ZONE_STRIP_H: f64 = 24.0;
    const STAGE_W: f64 = 1280.0;
    const STAGE_H: f64 = 720.0;

    // Palette (AxonOS Standard, Foundation Grande).
    const BG: &str = "#03070B";
    const PANEL2: &str = "#0B1520";
    const BORDER: &str = "#143140";
    const CYAN: &str = "#20D6FF";
    const CYAN_SOFT: &str = "#8EEBFF";
    const GREEN: &str = "#7CFF4F";
    const RED: &str = "#FF4D4D";
    const VIOLET: &str = "#9B7CFF";
    const AMBER: &str = "#FF9F1C";
    const MUTED: &str = "#8EA0B6";

    const MONO: &str = "Consolas, 'SF Mono', SFMono-Regular, Menlo, 'Liberation Mono', monospace";

    const STEP_MS: f64 = 1000.0 / TICKS_PER_SECOND as f64;
    const MAX_STEPS_PER_FRAME: u32 = 6;
    const MAX_MESSAGES: usize = 3;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Shell {
        Menu,
        Running,
        Paused,
        Help,
        Ended,
    }

    struct Ui {
        st_run: Element,
        st_trust: Element,
        st_risk: Element,
        st_integrity: Element,
        st_evidence: Element,
        st_consent: Element,
        bar_trust: HtmlElement,
        bar_risk: HtmlElement,
        bar_integrity: HtmlElement,
        consent_dot: Element,
        gates_line: Element,
        gate_pips: Vec<Element>,
        event_log: Element,
        seed_chip: Element,
        ov_menu: HtmlElement,
        ov_help: HtmlElement,
        ov_pause: HtmlElement,
        ov_end: HtmlElement,
        end_title: Element,
        end_body: Element,
        end_stats: Element,
        stage: HtmlElement,
        diff_chips: Vec<Element>,
        act_buttons: Vec<Element>,
    }

    struct Msg {
        text: String,
        class: &'static str,
    }

    struct App {
        game: GameState,
        shell: Shell,
        prev_shell: Shell,
        difficulty: Difficulty,
        seed: u64,
        run_no: u32,
        queued_lane: Option<u8>,
        queued_action: Option<Action>,
        msgs: VecDeque<Msg>,
        msgs_dirty: bool,
        canvas: HtmlCanvasElement,
        ctx: CanvasRenderingContext2d,
        dpr: f64,
        ui: Ui,
        acc: f64,
        last_ts: f64,
    }

    fn doc() -> Document {
        web_sys::window()
            .expect("window")
            .document()
            .expect("document")
    }

    fn el(document: &Document, id: &str) -> Element {
        document
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("missing #{id}"))
    }

    fn hel(document: &Document, id: &str) -> HtmlElement {
        el(document, id).dyn_into().expect("HtmlElement")
    }

    impl App {
        fn new(window: &Window) -> Result<Self, JsValue> {
            let document = window.document().expect("document");
            let canvas: HtmlCanvasElement = el(&document, "playfield").dyn_into()?;
            let ctx: CanvasRenderingContext2d = canvas
                .get_context("2d")?
                .ok_or_else(|| JsValue::from_str("missing 2d context"))?
                .dyn_into()?;

            let gate_pips = (0..5)
                .map(|i| el(&document, &format!("gate-{i}")))
                .collect();
            let act_buttons = ["act-1", "act-2", "act-3", "act-4", "act-5", "act-release"]
                .iter()
                .map(|id| el(&document, id))
                .collect();
            let diff_chips = ["diff-calm", "diff-standard", "diff-intense"]
                .iter()
                .map(|id| el(&document, id))
                .collect();

            let ui = Ui {
                st_run: el(&document, "st-run"),
                st_trust: el(&document, "st-trust"),
                st_risk: el(&document, "st-risk"),
                st_integrity: el(&document, "st-integrity"),
                st_evidence: el(&document, "st-evidence"),
                st_consent: el(&document, "st-consent"),
                bar_trust: hel(&document, "bar-trust"),
                bar_risk: hel(&document, "bar-risk"),
                bar_integrity: hel(&document, "bar-integrity"),
                consent_dot: el(&document, "consent-dot"),
                gates_line: el(&document, "gates-line"),
                gate_pips,
                event_log: el(&document, "event-log"),
                seed_chip: el(&document, "seed-chip"),
                ov_menu: hel(&document, "ov-menu"),
                ov_help: hel(&document, "ov-help"),
                ov_pause: hel(&document, "ov-pause"),
                ov_end: hel(&document, "ov-end"),
                end_title: el(&document, "end-title"),
                end_body: el(&document, "end-body"),
                end_stats: el(&document, "end-stats"),
                stage: hel(&document, "stage"),
                diff_chips,
                act_buttons,
            };

            let dpr = window.device_pixel_ratio().max(1.0);
            canvas.set_width((CANVAS_W * dpr) as u32);
            canvas.set_height((CANVAS_H * dpr) as u32);

            let mut app = Self {
                game: GameState::new(GameConfig::default()),
                shell: Shell::Menu,
                prev_shell: Shell::Menu,
                difficulty: Difficulty::Standard,
                seed: 0x2112,
                run_no: 0,
                queued_lane: None,
                queued_action: None,
                msgs: VecDeque::new(),
                msgs_dirty: true,
                canvas,
                ctx,
                dpr,
                ui,
                acc: 0.0,
                last_ts: 0.0,
            };
            app.apply_shell();
            app.update_dom();
            app.resize(window);
            app.render();
            Ok(app)
        }

        // -- shell ----------------------------------------------------------

        fn apply_shell(&mut self) {
            let show = |elem: &HtmlElement, on: bool| {
                let _ = elem
                    .style()
                    .set_property("display", if on { "grid" } else { "none" });
            };
            show(&self.ui.ov_menu, self.shell == Shell::Menu);
            show(&self.ui.ov_help, self.shell == Shell::Help);
            show(&self.ui.ov_pause, self.shell == Shell::Paused);
            show(&self.ui.ov_end, self.shell == Shell::Ended);
        }

        fn set_shell(&mut self, shell: Shell) {
            if self.shell != shell {
                self.prev_shell = self.shell;
                self.shell = shell;
                self.apply_shell();
            }
        }

        fn start_run(&mut self) {
            self.run_no += 1;
            self.seed = mix_seed(js_sys::Date::now(), self.run_no);
            self.game = GameState::new(GameConfig {
                seed: self.seed,
                difficulty: self.difficulty,
            });
            self.queued_lane = None;
            self.queued_action = None;
            self.msgs.clear();
            self.push_msg("RUN STARTED · HOLD THE BOUNDARY", "ok");
            self.acc = 0.0;
            self.set_shell(Shell::Running);
            self.update_dom();
        }

        fn end_run(&mut self) {
            let snapshot = self.game.snapshot();
            let victory = snapshot.status == Status::Victory;
            let title = if victory {
                "REVIEWER SURVIVAL ACHIEVED"
            } else {
                "BOUNDARY BREACH"
            };
            let body = if victory {
                "The boundary held.<br/>Raw signal stayed inside.<br/>Applications received typed intent only."
            } else {
                "Raw signal or unsafe claims crossed the boundary.<br/>Fix the evidence path. Seal the interface. Try again."
            };
            let _ = self.ui.end_title.set_attribute(
                "class",
                if victory {
                    "end-title victory"
                } else {
                    "end-title failure"
                },
            );
            self.ui.end_title.set_text_content(Some(title));
            self.ui.end_body.set_inner_html(body);
            self.ui.end_stats.set_text_content(Some(&format!(
                "TRUST {} · RISK {} · INTEGRITY {} · EVIDENCE {} · GATES {}/5 · RAW LEAKS {} · TICK {}",
                snapshot.trust,
                snapshot.risk,
                snapshot.integrity,
                snapshot.evidence_level.as_str(),
                snapshot.gates_passed,
                snapshot.raw_leaks,
                snapshot.tick
            )));
            self.set_shell(Shell::Ended);
        }

        // -- input ----------------------------------------------------------

        fn queue_action(&mut self, action: Action) {
            if self.shell == Shell::Running {
                self.queued_action = Some(action);
            }
        }

        fn move_lane(&mut self, delta: i8) {
            if self.shell != Shell::Running {
                return;
            }
            let current = self
                .queued_lane
                .unwrap_or(self.game.snapshot().selected_lane);
            let next = (current as i8 + delta).clamp(0, LANES as i8 - 1) as u8;
            self.queued_lane = Some(next);
        }

        fn select_lane(&mut self, lane: u8) {
            if self.shell == Shell::Running {
                self.queued_lane = Some(lane.min(LANES - 1));
            }
        }

        fn key(&mut self, event: &KeyboardEvent) {
            let key = event.key();
            match self.shell {
                Shell::Menu => match key.as_str() {
                    "Enter" | " " => {
                        event.prevent_default();
                        self.start_run();
                    }
                    "h" | "H" => self.set_shell(Shell::Help),
                    _ => {}
                },
                Shell::Help => match key.as_str() {
                    "Escape" | "h" | "H" => {
                        let back = self.prev_shell;
                        self.set_shell(back);
                    }
                    _ => {}
                },
                Shell::Paused => match key.as_str() {
                    "p" | "P" | "Escape" => self.set_shell(Shell::Running),
                    "r" | "R" => self.start_run(),
                    "h" | "H" => self.set_shell(Shell::Help),
                    _ => {}
                },
                Shell::Ended => match key.as_str() {
                    "Enter" | "r" | "R" | " " => {
                        event.prevent_default();
                        self.start_run();
                    }
                    "h" | "H" => self.set_shell(Shell::Help),
                    _ => {}
                },
                Shell::Running => match key.as_str() {
                    "ArrowUp" | "w" | "W" => {
                        event.prevent_default();
                        self.move_lane(-1);
                    }
                    "ArrowDown" | "s" | "S" => {
                        event.prevent_default();
                        self.move_lane(1);
                    }
                    "1" => self.queue_action(Action::Validate),
                    "2" => self.queue_action(Action::Convert),
                    "3" => self.queue_action(Action::Quarantine),
                    "4" => self.queue_action(Action::ConsentGate),
                    "5" => self.queue_action(Action::EvidenceGate),
                    "Enter" => {
                        event.prevent_default();
                        self.queue_action(Action::Release);
                    }
                    "p" | "P" | "Escape" => self.set_shell(Shell::Paused),
                    "h" | "H" => self.set_shell(Shell::Help),
                    "r" | "R" => self.start_run(),
                    _ => {}
                },
            }
        }

        fn command(&mut self, cmd: &str) {
            match cmd {
                "start" => self.start_run(),
                "restart" => self.start_run(),
                "help" => self.set_shell(Shell::Help),
                "close-help" => {
                    let back = self.prev_shell;
                    self.set_shell(back);
                }
                "pause" => {
                    if self.shell == Shell::Running {
                        self.set_shell(Shell::Paused);
                    } else if self.shell == Shell::Paused {
                        self.set_shell(Shell::Running);
                    }
                }
                "resume" => self.set_shell(Shell::Running),
                _ => {}
            }
        }

        fn set_difficulty(&mut self, name: &str) {
            if let Some(difficulty) = Difficulty::from_name(name) {
                self.difficulty = difficulty;
                for chip in &self.ui.diff_chips {
                    let id = chip.id();
                    let selected = id == format!("diff-{}", difficulty.name());
                    let _ = chip.class_list().toggle_with_force("selected", selected);
                }
            }
        }

        // -- simulation -----------------------------------------------------

        fn frame(&mut self, ts: f64) {
            if self.last_ts == 0.0 {
                self.last_ts = ts;
            }
            let dt = (ts - self.last_ts).min(250.0);
            self.last_ts = ts;

            if self.shell == Shell::Running {
                self.acc += dt;
                let mut steps = 0;
                while self.acc >= STEP_MS && steps < MAX_STEPS_PER_FRAME {
                    self.tick();
                    self.acc -= STEP_MS;
                    steps += 1;
                }
                if steps == MAX_STEPS_PER_FRAME {
                    self.acc = 0.0;
                }
                self.update_dom();
            }
            self.render();
        }

        fn tick(&mut self) {
            let input = Input {
                select_lane: self.queued_lane.take(),
                action: self.queued_action.take(),
            };
            self.game.step(input);
            let events: heapless_events::Buf = self.game.events().iter().collect();
            for event in events.iter() {
                self.handle_event(*event);
            }
            if self.game.status() != Status::Running && self.shell == Shell::Running {
                self.update_dom();
                self.end_run();
            }
        }

        fn handle_event(&mut self, event: CoreEvent) {
            let (text, class): (String, &'static str) = match event {
                CoreEvent::Validated(_) => ("INTENT VALIDATED · SCHEMA TYPED".into(), "ok"),
                CoreEvent::Revealed(kind) => (
                    format!("?PKT CLASSIFIED · {} · RISK +4", kind.label()),
                    "warn",
                ),
                CoreEvent::Converted => ("INTENT CONVERTED · TYPED FOR DELIVERY".into(), "ok"),
                CoreEvent::ConvertBlockedConsent => {
                    ("CONVERT BLOCKED · CONSENT GATE CLOSED".into(), "warn")
                }
                CoreEvent::ConvertBlockedEvidence => {
                    ("CONVERT BLOCKED · EVIDENCE BELOW L1".into(), "warn")
                }
                CoreEvent::QuarantineBlockedUnknown => {
                    ("QUARANTINE BLOCKED · CLASSIFY ?PKT FIRST".into(), "warn")
                }
                CoreEvent::Quarantined(kind) => {
                    (format!("HAZARD QUARANTINED · {}", kind.label()), "ok")
                }
                CoreEvent::FalsePositive(kind) => (
                    format!("FALSE POSITIVE · {} QUARANTINED · TRUST −3", kind.label()),
                    "bad",
                ),
                CoreEvent::ConsentOn => ("CONSENT GATE OPEN · 25s WINDOW".into(), "ok"),
                CoreEvent::ConsentExpired => ("CONSENT EXPIRED · GATE CLOSED".into(), "warn"),
                CoreEvent::EvidenceUp(level, points) => (
                    format!("EVIDENCE LOGGED · {} ({points} pts)", level.as_str()),
                    "ok",
                ),
                CoreEvent::GatePassed(index) => (
                    format!(
                        "REVIEW GATE PASSED · {}",
                        neural_boundary_core::gate_name(index)
                    ),
                    "ok",
                ),
                CoreEvent::Delivered => ("TYPED INTENT DELIVERED · TRUST +6".into(), "ok"),
                CoreEvent::MissedIntent => ("INTENT MISSED AT MEMBRANE · TRUST −1".into(), "warn"),
                CoreEvent::LostArtifact(kind) => {
                    (format!("ARTIFACT LOST · {}", kind.label()), "warn")
                }
                CoreEvent::MinorBreach(kind) => (
                    format!("CLAIM CROSSED · {} · INTEGRITY −6", kind.label()),
                    "bad",
                ),
                CoreEvent::MajorBreach(kind) => (
                    format!("MAJOR BREACH · {} · INTEGRITY −8 RISK +6", kind.label()),
                    "bad",
                ),
                CoreEvent::RawLeakBreach => {
                    ("RAW LEAK HIT THE MEMBRANE · INTEGRITY −10".into(), "bad")
                }
                CoreEvent::StimBreach => ("DIRECT STIM CROSSED · BOUNDARY BREACH".into(), "bad"),
                CoreEvent::ReleaseRejected(blocker) => (
                    format!("RELEASE REJECTED · {}", blocker_text(blocker)),
                    "bad",
                ),
                CoreEvent::ReleaseSealed => ("RELEASE ACCEPTED · BOUNDARY SEALED".into(), "ok"),
                CoreEvent::NoTarget(action) => (
                    format!("{} · NO TARGET IN GATE WINDOW", action_label(action)),
                    "warn",
                ),
                CoreEvent::Defeated(_) => return,
            };
            self.push_msg(&text, class);
        }

        fn push_msg(&mut self, text: &str, class: &'static str) {
            self.msgs.push_front(Msg {
                text: text.to_string(),
                class,
            });
            while self.msgs.len() > MAX_MESSAGES {
                self.msgs.pop_back();
            }
            self.msgs_dirty = true;
        }

        // -- DOM ------------------------------------------------------------

        fn update_dom(&mut self) {
            let snapshot = self.game.snapshot();

            self.ui
                .st_run
                .set_text_content(Some(&format!("{:02}", self.run_no.max(1))));
            self.ui
                .st_trust
                .set_text_content(Some(&snapshot.trust.to_string()));
            self.ui
                .st_risk
                .set_text_content(Some(&snapshot.risk.to_string()));
            self.ui
                .st_integrity
                .set_text_content(Some(&snapshot.integrity.to_string()));
            self.ui.st_evidence.set_text_content(Some(&format!(
                "{} · {} pts",
                snapshot.evidence_level.as_str(),
                snapshot.evidence_points
            )));

            let consent_text = if snapshot.consent_active {
                format!(
                    "OPEN · {}s",
                    (snapshot.consent_remaining + TICKS_PER_SECOND - 1) / TICKS_PER_SECOND
                )
            } else {
                "CLOSED".to_string()
            };
            self.ui.st_consent.set_text_content(Some(&consent_text));
            let _ = self
                .ui
                .consent_dot
                .class_list()
                .toggle_with_force("on", snapshot.consent_active);

            let set_bar = |bar: &HtmlElement, value: u8| {
                let _ = bar.style().set_property("width", &format!("{value}%"));
            };
            set_bar(&self.ui.bar_trust, snapshot.trust);
            set_bar(&self.ui.bar_risk, snapshot.risk);
            set_bar(&self.ui.bar_integrity, snapshot.integrity);

            self.ui
                .gates_line
                .set_text_content(Some(&format!("GATES PASSED {}/5", snapshot.gates_passed)));
            for (index, pip) in self.ui.gate_pips.iter().enumerate() {
                let passed = snapshot.gates_mask & (1 << index) != 0;
                let _ = pip.class_list().toggle_with_force("passed", passed);
            }

            for button in &self.ui.act_buttons {
                let _ = button
                    .class_list()
                    .toggle_with_force("cooling", snapshot.cooldown > 0);
            }

            self.ui
                .seed_chip
                .set_text_content(Some(&format!("SEED 0x{:08X}", (self.seed & 0xFFFF_FFFF))));

            if self.msgs_dirty {
                let html = self
                    .msgs
                    .iter()
                    .enumerate()
                    .map(|(index, msg)| {
                        format!(
                            "<span class=\"msg {} fade{}\">{}</span>",
                            msg.class, index, msg.text
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("");
                self.ui.event_log.set_inner_html(&html);
                self.msgs_dirty = false;
            }
        }

        fn resize(&mut self, window: &Window) {
            let vw = window
                .inner_width()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(STAGE_W);
            let vh = window
                .inner_height()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(STAGE_H);
            let scale = ((vw - 16.0) / STAGE_W)
                .min((vh - 16.0) / STAGE_H)
                .min(1.0)
                .max(0.2);
            let _ = self
                .ui
                .stage
                .style()
                .set_property("transform", &format!("scale({scale:.4})"));
        }

        // -- canvas ---------------------------------------------------------

        fn render(&self) {
            let ctx = &self.ctx;
            let dpr = self.dpr;
            ctx.save();
            let _ = ctx.scale(dpr, dpr);

            // Background.
            ctx.set_fill_style_str(BG);
            ctx.fill_rect(0.0, 0.0, CANVAS_W, CANVAS_H);

            let boundary_px = CANVAS_W * BOUNDARY_X as f64 / FIELD_END_X as f64;
            let window_px = CANVAS_W * GATE_WINDOW as f64 / FIELD_END_X as f64;
            let lane_top = ZONE_STRIP_H;
            let lane_h = (CANVAS_H - ZONE_STRIP_H) / LANES as f64;
            let snapshot = self.game.snapshot();

            // App zone tint.
            ctx.set_fill_style_str("rgba(124, 255, 79, 0.025)");
            ctx.fill_rect(
                boundary_px,
                lane_top,
                CANVAS_W - boundary_px,
                CANVAS_H - lane_top,
            );

            // Gate window tint.
            ctx.set_fill_style_str("rgba(32, 214, 255, 0.030)");
            ctx.fill_rect(
                boundary_px - window_px,
                lane_top,
                window_px,
                CANVAS_H - lane_top,
            );

            // Zone strip.
            ctx.set_fill_style_str(PANEL2);
            ctx.fill_rect(0.0, 0.0, CANVAS_W, ZONE_STRIP_H);
            ctx.set_stroke_style_str(BORDER);
            ctx.set_line_width(1.0);
            ctx.stroke_rect(0.5, 0.5, CANVAS_W - 1.0, ZONE_STRIP_H - 1.0);
            ctx.set_font(&format!("600 10px {MONO}"));
            ctx.set_text_baseline("middle");
            ctx.set_text_align("center");
            ctx.set_fill_style_str(MUTED);
            ctx.fill_text("SIGNAL ZONE", boundary_px * 0.5, ZONE_STRIP_H * 0.55)
                .ok();
            ctx.set_fill_style_str(CYAN);
            ctx.fill_text("BOUNDARY", boundary_px, ZONE_STRIP_H * 0.55)
                .ok();
            ctx.set_fill_style_str(GREEN);
            ctx.fill_text(
                "APP ZONE",
                boundary_px + (CANVAS_W - boundary_px) * 0.5,
                ZONE_STRIP_H * 0.55,
            )
            .ok();

            // Lane rows.
            for lane in 0..LANES {
                let y = lane_top + lane as f64 * lane_h;
                ctx.set_stroke_style_str("rgba(20, 49, 64, 0.55)");
                ctx.begin_path();
                ctx.move_to(0.0, y + 0.5);
                ctx.line_to(CANVAS_W, y + 0.5);
                ctx.stroke();
                if lane == snapshot.selected_lane {
                    ctx.set_fill_style_str("rgba(32, 214, 255, 0.045)");
                    ctx.fill_rect(0.0, y, CANVAS_W, lane_h);
                    ctx.set_fill_style_str(CYAN);
                    ctx.fill_rect(0.0, y + 3.0, 3.0, lane_h - 6.0);
                }
            }
            ctx.set_stroke_style_str("rgba(20, 49, 64, 0.55)");
            ctx.begin_path();
            ctx.move_to(0.0, CANVAS_H - 0.5);
            ctx.line_to(CANVAS_W, CANVAS_H - 0.5);
            ctx.stroke();

            // Membrane: double line.
            ctx.set_stroke_style_str(CYAN);
            ctx.set_line_width(2.0);
            ctx.begin_path();
            ctx.move_to(boundary_px - 2.0, lane_top);
            ctx.line_to(boundary_px - 2.0, CANVAS_H);
            ctx.stroke();
            ctx.set_stroke_style_str("rgba(32, 214, 255, 0.35)");
            ctx.set_line_width(1.0);
            ctx.begin_path();
            ctx.move_to(boundary_px + 3.0, lane_top);
            ctx.line_to(boundary_px + 3.0, CANVAS_H);
            ctx.stroke();

            // Application threshold.
            ctx.set_stroke_style_str("rgba(124, 255, 79, 0.30)");
            ctx.begin_path();
            ctx.move_to(CANVAS_W - 1.5, lane_top);
            ctx.line_to(CANVAS_W - 1.5, CANVAS_H);
            ctx.stroke();

            // Selected-lane gate bracket + cooldown.
            let sel_y = lane_top + snapshot.selected_lane as f64 * lane_h;
            ctx.set_stroke_style_str(CYAN_SOFT);
            ctx.set_line_width(2.0);
            let bx = boundary_px - 2.0;
            ctx.begin_path();
            ctx.move_to(bx - 8.0, sel_y + 5.0);
            ctx.line_to(bx, sel_y + 5.0);
            ctx.line_to(bx, sel_y + 15.0);
            ctx.move_to(bx - 8.0, sel_y + lane_h - 5.0);
            ctx.line_to(bx, sel_y + lane_h - 5.0);
            ctx.line_to(bx, sel_y + lane_h - 15.0);
            ctx.stroke();
            if snapshot.cooldown > 0 {
                let frac = snapshot.cooldown as f64 / neural_boundary_core::ACTION_COOLDOWN as f64;
                ctx.set_fill_style_str(AMBER);
                ctx.fill_rect(
                    bx - window_px,
                    sel_y + lane_h - 4.0,
                    window_px * frac.min(1.0),
                    2.0,
                );
            }

            // Entities.
            for entity in self.game.entities().iter().flatten() {
                self.draw_entity(entity, lane_top, lane_h);
            }

            ctx.restore();
        }

        fn draw_entity(&self, entity: &Entity, lane_top: f64, lane_h: f64) {
            let ctx = &self.ctx;
            let x = CANVAS_W * entity.x as f64 / FIELD_END_X as f64;
            let cy = lane_top + entity.lane as f64 * lane_h + lane_h * 0.5;
            let w = 84.0;
            let h = 26.0;
            let left = (x - w * 0.5).clamp(-w, CANVAS_W);
            let top = cy - h * 0.5;

            let (border, label_color, fill) = entity_colors(entity.kind);
            ctx.set_fill_style_str(fill);
            round_rect(ctx, left, top, w, h, 6.0);
            ctx.fill();
            ctx.set_stroke_style_str(border);
            ctx.set_line_width(1.5);
            round_rect(ctx, left, top, w, h, 6.0);
            ctx.stroke();

            ctx.set_font(&format!("700 11px {MONO}"));
            ctx.set_text_align("center");
            ctx.set_text_baseline("middle");
            ctx.set_fill_style_str(label_color);
            ctx.fill_text(entity.kind.label(), left + w * 0.5, cy + 0.5)
                .ok();

            if entity.validated && entity.kind == EntityKind::IntentCandidate {
                ctx.set_fill_style_str(GREEN);
                ctx.fill_rect(left + 4.0, top + h - 5.0, w - 8.0, 2.0);
            }
        }
    }

    fn entity_colors(kind: EntityKind) -> (&'static str, &'static str, &'static str) {
        match kind {
            EntityKind::TypedIntent => (GREEN, GREEN, "rgba(124, 255, 79, 0.08)"),
            EntityKind::IntentCandidate
            | EntityKind::ConsentToken
            | EntityKind::Evidence
            | EntityKind::Checksum
            | EntityKind::CiTest => (CYAN, CYAN_SOFT, "rgba(11, 21, 32, 0.92)"),
            EntityKind::UnknownPacket => (VIOLET, "#C9B8FF", "rgba(155, 124, 255, 0.10)"),
            EntityKind::Overclaim | EntityKind::NoTrace | EntityKind::RoadmapFact => {
                (AMBER, "#FFC46B", "rgba(255, 159, 28, 0.08)")
            }
            EntityKind::RawLeak
            | EntityKind::DirectStim
            | EntityKind::UnsafeBlock
            | EntityKind::Unbounded => (RED, "#FF8585", "rgba(255, 77, 77, 0.08)"),
        }
    }

    fn round_rect(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
        ctx.begin_path();
        ctx.move_to(x + r, y);
        ctx.line_to(x + w - r, y);
        ctx.quadratic_curve_to(x + w, y, x + w, y + r);
        ctx.line_to(x + w, y + h - r);
        ctx.quadratic_curve_to(x + w, y + h, x + w - r, y + h);
        ctx.line_to(x + r, y + h);
        ctx.quadratic_curve_to(x, y + h, x, y + h - r);
        ctx.line_to(x, y + r);
        ctx.quadratic_curve_to(x, y, x + r, y);
        ctx.close_path();
    }

    fn blocker_text(blocker: ReleaseBlocker) -> &'static str {
        blocker.as_str()
    }

    fn action_label(action: Action) -> &'static str {
        match action {
            Action::Validate => "VALIDATE",
            Action::Convert => "CONVERT",
            Action::Quarantine => "QUARANTINE",
            Action::ConsentGate => "CONSENT",
            Action::EvidenceGate => "EVIDENCE",
            Action::Release => "RELEASE",
        }
    }

    fn mix_seed(now_ms: f64, run: u32) -> u64 {
        let mut x = (now_ms as u64) ^ ((run as u64) << 48) ^ 0x9E37_79B9_7F4A_7C15;
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        if x == 0 {
            0x2112
        } else {
            x
        }
    }

    /// Tiny fixed-capacity event copy buffer (avoids borrowing `game` while
    /// mutating `self` during event handling).
    mod heapless_events {
        use neural_boundary_core::{Event, MAX_EVENTS};

        pub struct Buf {
            items: [Option<Event>; MAX_EVENTS],
            len: usize,
        }

        impl Buf {
            pub fn iter(&self) -> impl Iterator<Item = &Event> + '_ {
                self.items.iter().take(self.len).flatten()
            }
        }

        impl FromIterator<Event> for Buf {
            fn from_iter<T: IntoIterator<Item = Event>>(iter: T) -> Self {
                let mut buf = Buf {
                    items: [None; MAX_EVENTS],
                    len: 0,
                };
                for event in iter {
                    if buf.len < MAX_EVENTS {
                        buf.items[buf.len] = Some(event);
                        buf.len += 1;
                    }
                }
                buf
            }
        }
    }

    // -- bootstrap -----------------------------------------------------------

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
        let app = Rc::new(RefCell::new(App::new(&window)?));

        // Keyboard.
        {
            let app = Rc::clone(&app);
            let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
                app.borrow_mut().key(&event);
            });
            doc().add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        // Click delegation: [data-cmd], [data-act], [data-diff], canvas lanes.
        {
            let app = Rc::clone(&app);
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
                let Some(target) = event
                    .target()
                    .and_then(|target| target.dyn_into::<Element>().ok())
                else {
                    return;
                };
                if let Ok(Some(found)) = target.closest("[data-cmd]") {
                    if let Some(cmd) = found.get_attribute("data-cmd") {
                        app.borrow_mut().command(&cmd);
                        return;
                    }
                }
                if let Ok(Some(found)) = target.closest("[data-diff]") {
                    if let Some(diff) = found.get_attribute("data-diff") {
                        app.borrow_mut().set_difficulty(&diff);
                        return;
                    }
                }
                if let Ok(Some(found)) = target.closest("[data-act]") {
                    if let Some(act) = found.get_attribute("data-act") {
                        let action = match act.as_str() {
                            "1" => Some(Action::Validate),
                            "2" => Some(Action::Convert),
                            "3" => Some(Action::Quarantine),
                            "4" => Some(Action::ConsentGate),
                            "5" => Some(Action::EvidenceGate),
                            "release" => Some(Action::Release),
                            _ => None,
                        };
                        if let Some(action) = action {
                            app.borrow_mut().queue_action(action);
                        }
                    }
                }
            });
            doc().add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        // Canvas lane selection.
        {
            let app = Rc::clone(&app);
            let canvas = app.borrow().canvas.clone();
            let canvas_for_handler = canvas.clone();
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
                let rect = canvas_for_handler.get_bounding_client_rect();
                if rect.height() <= 0.0 {
                    return;
                }
                let y = (event.client_y() as f64 - rect.top()) / rect.height() * CANVAS_H;
                if y <= ZONE_STRIP_H {
                    return;
                }
                let lane_h = (CANVAS_H - ZONE_STRIP_H) / LANES as f64;
                let lane = ((y - ZONE_STRIP_H) / lane_h).floor().clamp(0.0, 4.0) as u8;
                app.borrow_mut().select_lane(lane);
            });
            canvas.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        // Resize.
        {
            let app = Rc::clone(&app);
            let closure =
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                    if let Some(window) = web_sys::window() {
                        app.borrow_mut().resize(&window);
                    }
                });
            window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        // requestAnimationFrame loop.
        {
            let app = Rc::clone(&app);
            let handle: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
            let handle_clone = Rc::clone(&handle);
            *handle.borrow_mut() = Some(Closure::new(move |ts: f64| {
                app.borrow_mut().frame(ts);
                if let Some(window) = web_sys::window() {
                    if let Some(closure) = handle_clone.borrow().as_ref() {
                        let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                    }
                }
            }));
            {
                let borrowed = handle.borrow();
                if let Some(closure) = borrowed.as_ref() {
                    window.request_animation_frame(closure.as_ref().unchecked_ref())?;
                }
            }
        }

        Ok(())
    }
}
