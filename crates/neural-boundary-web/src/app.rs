//! Application state machine. The deterministic core owns the game; this
//! struct owns presentation phase, queued input, overlays and persistence.

use crate::{accessibility, hud, render, storage};
use neural_boundary_core::{
    daily_seed, Action, Difficulty, Event, Input, RunMode, Simulation, SimulationConfig, Status,
    LANES, TICKS_PER_SECOND,
};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Element, HtmlElement, Window};

const STEP_MS: f64 = 1000.0 / TICKS_PER_SECOND as f64;
const MAX_STEPS_PER_FRAME: u32 = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Landing,
    Running,
    Paused,
    Result,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    Help,
    Pause,
    Blocked,
    Result,
    Reset,
}

pub struct App {
    pub simulation: Option<Simulation>,
    pub phase: Phase,
    pub modal: Option<Modal>,
    pub mode: RunMode,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub daily_date: Option<String>,
    run_counter: u32,
    queued_lane: Option<u8>,
    queued_action: Option<Action>,
    pub reduced_motion: bool,
    acc: f64,
    last_ts: f64,
    pub ui: hud::Ui,
    ctx: CanvasRenderingContext2d,
    dpr: f64,
}

impl App {
    pub fn new(window: &Window) -> Result<Self, wasm_bindgen::JsValue> {
        let document = window.document().expect("document");
        let ui = hud::Ui::new(&document);
        let ctx: CanvasRenderingContext2d = ui
            .canvas
            .get_context("2d")?
            .ok_or("missing 2d context")?
            .dyn_into()?;
        let dpr = window.device_pixel_ratio().max(1.0);
        let (stored_mode, difficulty) = storage::load_prefs();
        // First visit with no saved preferences and no completed guided run:
        // preselect the Guided onboarding mode.
        let mode = if !storage::has_prefs() && !storage::guided_done() {
            RunMode::Guided
        } else {
            stored_mode
        };

        let app = Self {
            simulation: None,
            phase: Phase::Landing,
            modal: None,
            mode,
            difficulty,
            seed: 0x3001,
            daily_date: None,
            run_counter: 0,
            queued_lane: None,
            queued_action: None,
            reduced_motion: false,
            acc: 0.0,
            last_ts: 0.0,
            ui,
            ctx,
            dpr,
        };
        app.sync_selectors();
        Ok(app)
    }

    // -- selectors -----------------------------------------------------------

    pub fn set_mode(&mut self, name: &str) {
        if let Some(mode) = RunMode::from_name(name) {
            self.mode = mode;
            storage::save_prefs(self.mode, self.difficulty);
            self.sync_selectors();
        }
    }

    pub fn set_difficulty(&mut self, name: &str) {
        if let Some(difficulty) = Difficulty::from_name(name) {
            self.difficulty = difficulty;
            storage::save_prefs(self.mode, self.difficulty);
            self.sync_selectors();
        }
    }

    fn sync_selectors(&self) {
        let document = self.ui.document();
        let sync = |selector: &str, attr: &str, current: &str| {
            if let Ok(list) = document.query_selector_all(selector) {
                for index in 0..list.length() {
                    if let Some(node) = list.item(index) {
                        if let Ok(element) = node.dyn_into::<Element>() {
                            let value = element.get_attribute(attr).unwrap_or_default();
                            let checked = value == current;
                            let _ = element.set_attribute(
                                "aria-checked",
                                if checked { "true" } else { "false" },
                            );
                        }
                    }
                }
            }
        };
        sync("[data-mode]", "data-mode", self.mode.name());
        sync(
            "[data-difficulty]",
            "data-difficulty",
            self.difficulty.name(),
        );
        self.ui.set_mode_blurb(self.mode);
    }

    // -- run lifecycle --------------------------------------------------------

    pub fn start_run(&mut self, reuse_seed: bool) {
        self.run_counter += 1;
        self.daily_date = None;
        if self.mode == RunMode::Daily {
            let now = js_sys::Date::new_0();
            let year = now.get_utc_full_year() as u16;
            let month = (now.get_utc_month() + 1) as u8;
            let day = now.get_utc_date() as u8;
            self.seed = daily_seed(year, month, day);
            self.daily_date = Some(format!("{year:04}-{month:02}-{day:02}"));
        } else if !reuse_seed {
            self.seed = mix_seed(js_sys::Date::now(), self.run_counter);
        }
        self.simulation = Some(Simulation::new(SimulationConfig {
            seed: self.seed,
            mode: self.mode,
            difficulty: self.difficulty,
        }));
        self.queued_lane = None;
        self.queued_action = None;
        self.acc = 0.0;
        self.close_modal();
        self.ui.reveal_game();
        self.ui.set_mode_chip(self.mode, self.difficulty);
        self.ui
            .set_seed(self.seed, self.mode, self.daily_date.as_deref());
        self.ui.set_pause_label(false);
        self.phase = Phase::Running;
        let opener = if self.mode == RunMode::Guided {
            "GUIDED RUN · the boundary will coach you"
        } else {
            "RUN STARTED · hold the boundary"
        };
        self.ui.set_feedback(opener, hud::Tone::Ok);
        accessibility::announce(
            self.ui.document(),
            &format!("Run started. Mode {}.", self.mode.name()),
        );
    }

    pub fn return_to_menu(&mut self) {
        self.simulation = None;
        self.phase = Phase::Landing;
        self.close_modal();
        self.ui.show_landing();
        self.sync_selectors();
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Paused;
            self.ui.set_pause_label(true);
            self.open_modal(Modal::Pause);
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Running;
            self.ui.set_pause_label(false);
            self.close_modal();
            self.last_ts = 0.0;
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.phase {
            Phase::Running => self.pause(),
            Phase::Paused => self.resume(),
            _ => {}
        }
    }

    pub fn auto_pause(&mut self) {
        if self.phase == Phase::Running {
            self.pause();
            self.ui
                .set_feedback("AUTO-PAUSED · tab went to background", hud::Tone::Warn);
        }
    }

    pub fn confirm_reset(&mut self) {
        storage::reset_all();
        let (stored_mode, difficulty) = storage::load_prefs();
        // First visit with no saved preferences and no completed guided run:
        // preselect the Guided onboarding mode.
        let mode = if !storage::has_prefs() && !storage::guided_done() {
            RunMode::Guided
        } else {
            stored_mode
        };
        self.mode = mode;
        self.difficulty = difficulty;
        self.sync_selectors();
        self.close_modal();
        accessibility::announce(self.ui.document(), "Local data cleared.");
    }

    // -- input ---------------------------------------------------------------

    pub fn move_lane(&mut self, delta: i8) {
        if self.phase != Phase::Running {
            return;
        }
        let current = self
            .queued_lane
            .or_else(|| {
                self.simulation
                    .as_ref()
                    .map(|sim| sim.snapshot().selected_lane)
            })
            .unwrap_or(2);
        let next = (current as i8 + delta).clamp(0, LANES as i8 - 1) as u8;
        self.queued_lane = Some(next);
    }

    pub fn select_lane(&mut self, lane: u8) {
        if self.phase == Phase::Running {
            self.queued_lane = Some(lane.min(LANES - 1));
        }
    }

    pub fn queue_action(&mut self, action: Action) {
        if self.phase == Phase::Running && self.modal.is_none() {
            self.queued_action = Some(action);
        }
    }

    // -- modal management ------------------------------------------------------

    pub fn modal_element(&self, modal: Modal) -> Option<Element> {
        let element: &HtmlElement = match modal {
            Modal::Help => &self.ui.ov_help,
            Modal::Pause => &self.ui.ov_pause,
            Modal::Blocked => &self.ui.ov_blocked,
            Modal::Result => &self.ui.ov_result,
            Modal::Reset => &self.ui.ov_reset,
        };
        Some(element.clone().unchecked_into())
    }

    pub fn open_modal(&mut self, modal: Modal) {
        self.close_modal();
        self.modal = Some(modal);
        if let Some(element) = self.modal_element(modal) {
            let _ = element.remove_attribute("hidden");
            accessibility::focus_first(&element);
        }
    }

    pub fn close_modal(&mut self) {
        if let Some(modal) = self.modal.take() {
            if let Some(element) = self.modal_element(modal) {
                let _ = element.set_attribute("hidden", "");
            }
            if modal == Modal::Pause && self.phase == Phase::Paused {
                // Closing the pause sheet resumes the run.
                self.phase = Phase::Running;
                self.ui.set_pause_label(false);
                self.last_ts = 0.0;
            }
        }
    }

    // -- frame loop ------------------------------------------------------------

    pub fn frame(&mut self, ts: f64) {
        if self.last_ts == 0.0 {
            self.last_ts = ts;
        }
        let dt = (ts - self.last_ts).min(250.0);
        self.last_ts = ts;

        if self.phase == Phase::Running && self.modal.is_none() {
            self.acc += dt;
            let mut steps = 0;
            while self.acc >= STEP_MS && steps < MAX_STEPS_PER_FRAME {
                self.tick();
                self.acc -= STEP_MS;
                steps += 1;
                if self.phase != Phase::Running {
                    break;
                }
            }
            if steps == MAX_STEPS_PER_FRAME {
                self.acc = 0.0;
            }
        }

        if let Some(simulation) = &self.simulation {
            if self.phase != Phase::Landing {
                self.ui.update(simulation);
                render::draw(
                    &self.ctx,
                    &self.ui.canvas,
                    simulation,
                    self.dpr,
                    self.reduced_motion,
                );
            }
        }
    }

    fn tick(&mut self) {
        let input = Input {
            select_lane: self.queued_lane.take(),
            action: self.queued_action.take(),
        };
        let (events, blockers, terminal) = {
            let Some(simulation) = self.simulation.as_mut() else {
                return;
            };
            simulation.step(input);
            let events: Vec<Event> = simulation.events().collect();
            let blocked = events
                .iter()
                .any(|event| matches!(event, Event::ReleaseRejected(_)));
            let blockers: Vec<_> = if blocked {
                let (slots, count) = simulation.release_blockers();
                slots
                    .iter()
                    .take(count as usize)
                    .flatten()
                    .copied()
                    .collect()
            } else {
                Vec::new()
            };
            (events, blockers, simulation.status() != Status::Running)
        };

        for event in &events {
            if let Some((line, tone, announce)) = hud::event_copy(*event) {
                self.ui.set_feedback(&line, tone);
                if let Some(text) = announce {
                    accessibility::announce(self.ui.document(), &text);
                }
            }
        }

        if !blockers.is_empty() {
            self.ui.fill_blocked(&blockers);
            self.open_modal(Modal::Blocked);
        }

        if terminal {
            self.finish_run();
        }
    }

    fn finish_run(&mut self) {
        let Some((mode, score, sealed_guided, grade)) =
            self.simulation.as_ref().map(|simulation| {
                let snapshot = simulation.snapshot();
                (
                    snapshot.mode,
                    snapshot.score,
                    snapshot.mode == RunMode::Guided
                        && snapshot.status
                            == Status::Terminal(neural_boundary_core::TerminalReason::Sealed),
                    simulation.grade().name(),
                )
            })
        else {
            return;
        };
        storage::save_best(mode, score);
        if sealed_guided {
            storage::mark_guided_done();
        }
        if let Some(simulation) = &self.simulation {
            self.ui
                .fill_result(simulation, self.seed, self.daily_date.as_deref());
        }
        self.phase = Phase::Result;
        self.open_modal(Modal::Result);
        accessibility::announce(
            self.ui.document(),
            &format!("Run finished. Grade {grade}. Score {score}."),
        );
    }

    pub fn canvas_click(&mut self, css_x: f64, css_y: f64) {
        let _ = css_x;
        let rect = self.ui.canvas.get_bounding_client_rect();
        if let Some(lane) = render::lane_from_css_y(rect.width(), rect.height(), css_y) {
            self.select_lane(lane);
        }
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
        0x3001
    } else {
        x
    }
}
