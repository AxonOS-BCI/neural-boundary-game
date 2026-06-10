#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub fn native_placeholder() -> &'static str {
    "neural-boundary-web is intended for wasm32 builds"
}

#[cfg(target_arch = "wasm32")]
#[allow(deprecated)]
mod wasm_app {
    use neural_boundary_core::{
        EntityKind, GameConfig, GamePhase, GameState, GateStatus, PlayerAction,
    };
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent, MouseEvent};

    const W: f64 = 1280.0;
    const H: f64 = 720.0;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();

        let window = window().ok_or_else(|| JsValue::from_str("missing window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("missing document"))?;
        let canvas = document
            .get_element_by_id("nbg-canvas")
            .ok_or_else(|| JsValue::from_str("missing canvas"))?
            .dyn_into::<HtmlCanvasElement>()?;
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("missing 2d"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let game = Rc::new(RefCell::new(GameState::new(GameConfig::default())));
        let pending = Rc::new(RefCell::new(PlayerAction::Idle));

        {
            let pending = Rc::clone(&pending);
            let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event| {
                *pending.borrow_mut() = key_to_action(&event.key());
                if event.key() == " " {
                    event.prevent_default();
                }
            });
            document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        {
            let pending = Rc::clone(&pending);
            let canvas_for_click = canvas.clone();
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |event| {
                let rect = canvas_for_click.get_bounding_client_rect();
                let sx = W / rect.width();
                let sy = H / rect.height();
                let x = (f64::from(event.client_x()) - rect.left()) * sx;
                let y = (f64::from(event.client_y()) - rect.top()) * sy;
                *pending.borrow_mut() = pointer_action(x, y);
            });
            canvas
                .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        type RafCallback = Closure<dyn FnMut()>;

        let raf: Rc<RefCell<Option<RafCallback>>> = Rc::new(RefCell::new(None));
        let raf2 = Rc::clone(&raf);
        let win = window.clone();

        *raf2.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
            let action = {
                let mut pending_action = pending.borrow_mut();
                let action = *pending_action;
                *pending_action = PlayerAction::Idle;
                action
            };

            game.borrow_mut().step(action);
            draw(&ctx, &game.borrow());

            if let Some(callback) = raf.borrow().as_ref() {
                let _ = win.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }));

        if let Some(callback) = raf2.borrow().as_ref() {
            window.request_animation_frame(callback.as_ref().unchecked_ref())?;
        }

        Ok(())
    }

    fn key_to_action(key: &str) -> PlayerAction {
        match key {
            "w" | "W" | "ArrowUp" => PlayerAction::MoveUp,
            "s" | "S" | "ArrowDown" => PlayerAction::MoveDown,
            "1" | " " => PlayerAction::Validate,
            "2" => PlayerAction::Convert,
            "3" => PlayerAction::Quarantine,
            "4" => PlayerAction::ConsentGate,
            "5" => PlayerAction::EvidenceGate,
            "Enter" => PlayerAction::Release,
            "h" | "H" => PlayerAction::ToggleHelp,
            "r" | "R" => PlayerAction::Restart,
            "Escape" => PlayerAction::Pause,
            _ => PlayerAction::Idle,
        }
    }

    fn pointer_action(x: f64, y: f64) -> PlayerAction {
        if (612.0..674.0).contains(&y) {
            let idx = ((x - 72.0) / 188.0).floor() as i32;
            return match idx {
                0 => PlayerAction::Validate,
                1 => PlayerAction::Convert,
                2 => PlayerAction::Quarantine,
                3 => PlayerAction::ConsentGate,
                4 => PlayerAction::EvidenceGate,
                5 => PlayerAction::Release,
                _ => PlayerAction::Idle,
            };
        }

        if (112.0..146.0).contains(&y) && (1025.0..1110.0).contains(&x) {
            return PlayerAction::ToggleHelp;
        }

        if (112.0..146.0).contains(&y) && (1125.0..1218.0).contains(&x) {
            return PlayerAction::Restart;
        }

        if x < 930.0 && (196.0..560.0).contains(&y) {
            if y < 378.0 {
                PlayerAction::MoveUp
            } else {
                PlayerAction::MoveDown
            }
        } else {
            PlayerAction::Idle
        }
    }

    fn draw(ctx: &CanvasRenderingContext2d, game: &GameState) {
        let snapshot = game.snapshot();

        clear(ctx);
        background_grid(ctx);
        header(ctx);
        status_rail(ctx, game);
        playfield(ctx, game);
        principle_panel(ctx, game);
        action_bar(ctx);
        footer(ctx, game);

        match snapshot.phase {
            GamePhase::Menu => modal(
                ctx,
                "NEURAL BOUNDARY GAME",
                "Do not ship raw signal. Ship typed intent.",
                "Press any action or click the action bar to start.",
            ),
            GamePhase::Paused => modal(
                ctx,
                "PAUSED",
                "Deterministic state is frozen.",
                "Press Esc to resume.",
            ),
            GamePhase::Victory => modal(
                ctx,
                "REVIEWER SURVIVAL ACHIEVED",
                "The boundary held. Apps received typed intent only.",
                "Press R to restart.",
            ),
            GamePhase::Failure => modal(
                ctx,
                "BOUNDARY BREACH",
                "Raw signal or unsafe claims crossed the boundary.",
                "Press R to restart.",
            ),
            GamePhase::Running => {
                if snapshot.help_visible {
                    modal(
                        ctx,
                        "BOUNDARY RULES",
                        "Validate evidence, quarantine hazards, activate consent, and release only after gates pass.",
                        "1 Validate · 2 Convert · 3 Quarantine · 4 Consent · 5 Evidence · Enter Release",
                    );
                }
            }
        }
    }

    fn clear(ctx: &CanvasRenderingContext2d) {
        ctx.set_fill_style(&JsValue::from_str("#03070B"));
        ctx.fill_rect(0.0, 0.0, W, H);
    }

    fn background_grid(ctx: &CanvasRenderingContext2d) {
        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.040)"));
        ctx.set_line_width(1.0);

        for x in (80..1200).step_by(80) {
            ctx.begin_path();
            ctx.move_to(f64::from(x), 172.0);
            ctx.line_to(f64::from(x), 570.0);
            ctx.stroke();
        }

        for y in (190..560).step_by(48) {
            ctx.begin_path();
            ctx.move_to(64.0, f64::from(y));
            ctx.line_to(1216.0, f64::from(y));
            ctx.stroke();
        }

        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.075)"));
        ctx.begin_path();
        ctx.move_to(160.0, 76.0);
        ctx.line_to(1120.0, 76.0);
        ctx.stroke();
    }

    fn header(ctx: &CanvasRenderingContext2d) {
        ctx.set_text_align("center");
        ctx.set_font("800 42px Inter, system-ui, sans-serif");
        ctx.set_fill_style(&JsValue::from_str("#EDF5FF"));
        let _ = ctx.fill_text("NEURAL BOUNDARY GAME", 640.0, 78.0);

        ctx.set_font("600 15px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text("Do not ship raw signal. Ship typed intent.", 640.0, 112.0);

        ctx.set_text_align("left");
        chip(ctx, 72.0, 112.0, "AxonOS-BCI", "#20D6FF");
        chip(ctx, 196.0, 112.0, "Rust/WASM", "#8EEBFF");
        chip(ctx, 316.0, 112.0, "no_std core", "#7CFF4F");
        chip(ctx, 1025.0, 112.0, "Help", "#A8B4C4");
        chip(ctx, 1125.0, 112.0, "Restart", "#FF9F1C");
    }

    fn status_rail(ctx: &CanvasRenderingContext2d, game: &GameState) {
        let s = game.snapshot();
        let y = 158.0;
        panel(ctx, 72.0, y, 1136.0, 44.0, 18.0);
        ctx.set_font("700 13px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text("RUN", 96.0, y + 28.0);
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text(&format!("{:04}", s.tick), 134.0, y + 28.0);

        metric_inline(ctx, 250.0, y + 14.0, "TRUST", s.trust, "#7CFF4F");
        metric_inline(ctx, 445.0, y + 14.0, "RISK", s.risk, "#FF4D4D");
        metric_inline(ctx, 640.0, y + 14.0, "INTEGRITY", s.integrity, "#20D6FF");

        ctx.set_fill_style(&JsValue::from_str("#9B7CFF"));
        let _ = ctx.fill_text(
            &format!("EVIDENCE {}", s.evidence_level.as_str()),
            845.0,
            y + 28.0,
        );

        ctx.set_fill_style(&JsValue::from_str(if s.consent_active {
            "#7CFF4F"
        } else {
            "#8EA0B6"
        }));
        let _ = ctx.fill_text(
            if s.consent_active {
                "CONSENT ACTIVE"
            } else {
                "CONSENT IDLE"
            },
            1010.0,
            y + 28.0,
        );
    }

    fn playfield(ctx: &CanvasRenderingContext2d, game: &GameState) {
        panel(ctx, 72.0, 222.0, 820.0, 354.0, 24.0);

        ctx.set_font("700 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text("SIGNAL", 102.0, 250.0);
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text("BOUNDARY", 410.0, 250.0);
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text("APP", 798.0, 250.0);

        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.14)"));
        for lane in 0..5 {
            let y = lane_y(lane);
            ctx.begin_path();
            ctx.move_to(112.0, y);
            ctx.line_to(852.0, y);
            ctx.stroke();
        }

        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.55)"));
        ctx.set_line_width(2.0);
        ctx.begin_path();
        ctx.move_to(438.0, 274.0);
        ctx.line_to(438.0, 532.0);
        ctx.stroke();
        ctx.set_line_width(1.0);

        ctx.set_stroke_style(&JsValue::from_str("rgba(255,159,28,.28)"));
        ctx.begin_path();
        ctx.move_to(782.0, 274.0);
        ctx.line_to(782.0, 532.0);
        ctx.stroke();

        let py = lane_y(game.snapshot().player_lane);
        ctx.set_fill_style(&JsValue::from_str("rgba(32,214,255,.18)"));
        round(ctx, 409.0, py - 24.0, 58.0, 48.0, 16.0);
        ctx.fill();
        ctx.set_stroke_style(&JsValue::from_str("#20D6FF"));
        ctx.stroke();

        ctx.set_text_align("center");
        ctx.set_font("800 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text("GATE", 438.0, py + 5.0);
        ctx.set_text_align("left");

        for entity in game.entities().iter().flatten() {
            let x = 112.0 + (f64::from(entity.x) - 90.0) * 0.72;
            let y = lane_y(entity.lane);
            entity_card(ctx, x, y, entity.kind);
        }
    }

    fn principle_panel(ctx: &CanvasRenderingContext2d, game: &GameState) {
        panel(ctx, 920.0, 222.0, 288.0, 354.0, 24.0);

        ctx.set_font("800 14px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text("BOUNDARY PRINCIPLE", 950.0, 255.0);

        ctx.set_font("500 18px Inter, system-ui, sans-serif");
        ctx.set_fill_style(&JsValue::from_str("#EDF5FF"));
        let _ = ctx.fill_text("Raw signal stays", 950.0, 294.0);
        let _ = ctx.fill_text("inside the device.", 950.0, 320.0);

        ctx.set_font("600 13px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text("APPS RECEIVE", 950.0, 366.0);
        pill_line(ctx, 950.0, 386.0, "typed intent", "#7CFF4F");
        pill_line(ctx, 950.0, 420.0, "consent state", "#20D6FF");
        pill_line(ctx, 950.0, 454.0, "confidence", "#9B7CFF");

        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text("APPS NEVER RECEIVE", 950.0, 504.0);
        pill_line(ctx, 950.0, 524.0, "raw frames", "#FF4D4D");

        let s = game.snapshot();
        ctx.set_font("700 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text(&format!("gates passed {}/5", s.gates_passed), 1090.0, 548.0);
    }

    fn action_bar(ctx: &CanvasRenderingContext2d) {
        let actions = [
            ("1", "Validate", "#20D6FF"),
            ("2", "Convert", "#7CFF4F"),
            ("3", "Quarantine", "#FF4D4D"),
            ("4", "Consent", "#8EEBFF"),
            ("5", "Evidence", "#9B7CFF"),
            ("Enter", "Release", "#FF9F1C"),
        ];

        let mut x = 72.0;
        for (key, label, color) in actions {
            ctx.set_fill_style(&JsValue::from_str("rgba(7,16,24,.96)"));
            round(ctx, x, 612.0, 170.0, 62.0, 18.0);
            ctx.fill();
            ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.16)"));
            ctx.stroke();

            ctx.set_font("800 13px Consolas, monospace");
            ctx.set_fill_style(&JsValue::from_str(color));
            let _ = ctx.fill_text(key, x + 18.0, 636.0);

            ctx.set_font("700 16px Inter, system-ui, sans-serif");
            ctx.set_fill_style(&JsValue::from_str("#EDF5FF"));
            let _ = ctx.fill_text(label, x + 18.0, 660.0);

            x += 188.0;
        }
    }

    fn footer(ctx: &CanvasRenderingContext2d, game: &GameState) {
        let s = game.snapshot();
        let phase = match s.phase {
            GamePhase::Menu => "MENU",
            GamePhase::Running => "RUNNING",
            GamePhase::Paused => "PAUSED",
            GamePhase::Victory => "REVIEWER SURVIVAL",
            GamePhase::Failure => "BOUNDARY BREACH",
        };

        ctx.set_font("600 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#617186"));
        ctx.set_text_align("center");
        let _ = ctx.fill_text(
            &format!(
                "v1.0.3  •  phase {}  •  raw leaks {}  •  deterministic Rust core",
                phase, s.raw_leaks
            ),
            640.0,
            704.0,
        );
        ctx.set_text_align("left");
    }

    fn modal(ctx: &CanvasRenderingContext2d, title: &str, line1: &str, line2: &str) {
        ctx.set_fill_style(&JsValue::from_str("rgba(3,7,11,.82)"));
        ctx.fill_rect(0.0, 0.0, W, H);

        ctx.set_fill_style(&JsValue::from_str("rgba(7,16,24,.98)"));
        round(ctx, 250.0, 232.0, 780.0, 228.0, 28.0);
        ctx.fill();
        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.34)"));
        ctx.stroke();

        ctx.set_text_align("center");
        ctx.set_font("800 31px Inter, system-ui, sans-serif");
        ctx.set_fill_style(&JsValue::from_str("#EDF5FF"));
        let _ = ctx.fill_text(title, 640.0, 296.0);

        ctx.set_font("500 19px Inter, system-ui, sans-serif");
        ctx.set_fill_style(&JsValue::from_str("#A8B4C4"));
        let _ = ctx.fill_text(line1, 640.0, 346.0);

        ctx.set_font("700 15px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#20D6FF"));
        let _ = ctx.fill_text(line2, 640.0, 395.0);
        ctx.set_text_align("left");
    }

    fn entity_card(ctx: &CanvasRenderingContext2d, x: f64, y: f64, kind: EntityKind) {
        let color = entity_color(kind);
        ctx.set_fill_style(&JsValue::from_str("rgba(7,16,24,.94)"));
        round(ctx, x, y - 18.0, 126.0, 36.0, 13.0);
        ctx.fill();
        ctx.set_stroke_style(&JsValue::from_str(color));
        ctx.stroke();

        ctx.set_font("800 11px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str(color));
        let _ = ctx.fill_text(entity_label(kind), x + 12.0, y + 4.0);
    }

    fn metric_inline(
        ctx: &CanvasRenderingContext2d,
        x: f64,
        y: f64,
        label: &str,
        value: u8,
        color: &str,
    ) {
        ctx.set_font("700 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str("#8EA0B6"));
        let _ = ctx.fill_text(label, x, y + 17.0);

        ctx.set_fill_style(&JsValue::from_str("rgba(255,255,255,.055)"));
        round(ctx, x + 78.0, y + 6.0, 96.0, 10.0, 5.0);
        ctx.fill();

        ctx.set_fill_style(&JsValue::from_str(color));
        round(ctx, x + 78.0, y + 6.0, f64::from(value) * 0.96, 10.0, 5.0);
        ctx.fill();

        ctx.set_fill_style(&JsValue::from_str("#EDF5FF"));
        let _ = ctx.fill_text(&format!("{value:03}"), x + 184.0, y + 17.0);
    }

    fn chip(ctx: &CanvasRenderingContext2d, x: f64, y: f64, text: &str, color: &str) {
        let width = 46.0 + text.len() as f64 * 7.5;
        ctx.set_fill_style(&JsValue::from_str("rgba(7,16,24,.94)"));
        round(ctx, x, y, width, 34.0, 17.0);
        ctx.fill();
        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.14)"));
        ctx.stroke();

        ctx.set_font("700 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str(color));
        let _ = ctx.fill_text(text, x + 16.0, y + 22.0);
    }

    fn pill_line(ctx: &CanvasRenderingContext2d, x: f64, y: f64, text: &str, color: &str) {
        ctx.set_fill_style(&JsValue::from_str("rgba(255,255,255,.035)"));
        round(ctx, x, y, 214.0, 24.0, 12.0);
        ctx.fill();

        ctx.set_font("700 12px Consolas, monospace");
        ctx.set_fill_style(&JsValue::from_str(color));
        let _ = ctx.fill_text(text, x + 13.0, y + 17.0);
    }

    fn panel(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
        ctx.set_fill_style(&JsValue::from_str("rgba(7,16,24,.94)"));
        round(ctx, x, y, w, h, r);
        ctx.fill();
        ctx.set_stroke_style(&JsValue::from_str("rgba(32,214,255,.17)"));
        ctx.stroke();
    }

    fn round(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
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

    fn lane_y(lane: u8) -> f64 {
        304.0 + f64::from(lane) * 49.0
    }

    fn entity_color(kind: EntityKind) -> &'static str {
        match kind {
            EntityKind::RawLeak | EntityKind::DirectStim | EntityKind::UnsafeBlock => "#FF4D4D",
            EntityKind::Overclaim | EntityKind::NoTrace | EntityKind::RoadmapFact => "#FF9F1C",
            EntityKind::Evidence | EntityKind::Checksum | EntityKind::CiTest => "#9B7CFF",
            EntityKind::ConsentToken | EntityKind::TypedIntent | EntityKind::IntentCandidate => {
                "#7CFF4F"
            }
            _ => "#20D6FF",
        }
    }

    fn entity_label(kind: EntityKind) -> &'static str {
        match kind {
            EntityKind::IntentCandidate => "INTENT",
            EntityKind::ConsentToken => "CONSENT",
            EntityKind::Evidence => "EVIDENCE",
            EntityKind::Checksum => "SHA256",
            EntityKind::CiTest => "CI",
            EntityKind::TypedIntent => "TYPED",
            EntityKind::RawLeak => "RAW",
            EntityKind::DirectStim => "STIM",
            EntityKind::UnsafeBlock => "UNSAFE",
            EntityKind::Unbounded => "UNBOUNDED",
            EntityKind::Overclaim => "CLAIM",
            EntityKind::NoTrace => "NO TRACE",
            EntityKind::RoadmapFact => "ROADMAP",
            EntityKind::UnknownPacket => "UNKNOWN",
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_app::start;
