//! Canvas presentation of authoritative snapshots. Pure read-side: nothing
//! here mutates game state.

use neural_boundary_core::{
    EntityKind, RiskCategory, Simulation, ACTION_COOLDOWN, BOUNDARY_X, FIELD_END_X, GATE_WINDOW,
    LANES,
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Logical field size. Geometry is documented in `docs/GAME_SPEC.md`.
pub const LOGICAL_W: f64 = 1000.0;
pub const LOGICAL_H: f64 = 520.0;
const ZONE_STRIP: f64 = 26.0;

const BG: &str = "#030507";
const LINE_SUBTLE: &str = "rgba(255,255,255,0.10)";
const TEXT_DIM: &str = "rgba(244,241,232,0.42)";
const CYAN: &str = "#79def5";
const GOLD: &str = "#d6b96b";
const SAFE: &str = "#78e6ad";
const DANGER: &str = "#ff7186";
const SPECIAL: &str = "#a993ff";
const MONO: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// Letterboxed mapping from CSS pixels to the logical field.
#[derive(Clone, Copy, Debug)]
pub struct FieldMetrics {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

pub fn metrics(css_w: f64, css_h: f64) -> FieldMetrics {
    let scale = (css_w / LOGICAL_W).min(css_h / LOGICAL_H).max(0.05);
    FieldMetrics {
        scale,
        offset_x: (css_w - LOGICAL_W * scale) / 2.0,
        offset_y: (css_h - LOGICAL_H * scale) / 2.0,
    }
}

/// Resize the backing store to the element's CSS size at the device pixel
/// ratio. Returns the CSS size.
pub fn fit_canvas(canvas: &HtmlCanvasElement, dpr: f64) -> (f64, f64) {
    let rect = canvas.get_bounding_client_rect();
    let css_w = rect.width().max(1.0);
    let css_h = rect.height().max(1.0);
    let backing_w = (css_w * dpr) as u32;
    let backing_h = (css_h * dpr) as u32;
    if canvas.width() != backing_w {
        canvas.set_width(backing_w);
    }
    if canvas.height() != backing_h {
        canvas.set_height(backing_h);
    }
    (css_w, css_h)
}

pub fn lane_from_css_y(css_w: f64, css_h: f64, y: f64) -> Option<u8> {
    let field = metrics(css_w, css_h);
    let logical_y = (y - field.offset_y) / field.scale;
    if !(ZONE_STRIP..LOGICAL_H).contains(&logical_y) {
        return None;
    }
    let lane_h = (LOGICAL_H - ZONE_STRIP) / LANES as f64;
    Some((((logical_y - ZONE_STRIP) / lane_h) as u8).min(LANES - 1))
}

#[allow(clippy::too_many_lines)]
pub fn draw(
    ctx: &CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
    simulation: &Simulation,
    dpr: f64,
    reduced_motion: bool,
) {
    let (css_w, css_h) = fit_canvas(canvas, dpr);
    let field = metrics(css_w, css_h);
    let snapshot = simulation.snapshot();

    ctx.save();
    let _ = ctx.scale(dpr, dpr);
    ctx.set_fill_style_str(BG);
    ctx.fill_rect(0.0, 0.0, css_w, css_h);
    let _ = ctx.translate(field.offset_x, field.offset_y);
    let _ = ctx.scale(field.scale, field.scale);

    let boundary_x = LOGICAL_W * BOUNDARY_X as f64 / FIELD_END_X as f64;
    let window_w = LOGICAL_W * GATE_WINDOW as f64 / FIELD_END_X as f64;
    let lane_top = ZONE_STRIP;
    let lane_h = (LOGICAL_H - ZONE_STRIP) / LANES as f64;

    // Zone tints.
    ctx.set_fill_style_str("rgba(121,222,245,0.035)");
    ctx.fill_rect(
        boundary_x - window_w,
        lane_top,
        window_w,
        LOGICAL_H - lane_top,
    );
    ctx.set_fill_style_str("rgba(120,230,173,0.030)");
    ctx.fill_rect(
        boundary_x,
        lane_top,
        LOGICAL_W - boundary_x,
        LOGICAL_H - lane_top,
    );

    // Zone captions.
    ctx.set_text_baseline("middle");
    ctx.set_text_align("center");
    ctx.set_font(&format!("600 13px {MONO}"));
    ctx.set_fill_style_str(TEXT_DIM);
    let _ = ctx.fill_text("SIGNAL", boundary_x * 0.45, ZONE_STRIP * 0.55);
    ctx.set_fill_style_str(CYAN);
    let _ = ctx.fill_text("BOUNDARY", boundary_x, ZONE_STRIP * 0.55);
    ctx.set_fill_style_str(SAFE);
    let _ = ctx.fill_text(
        "APPLICATION",
        boundary_x + (LOGICAL_W - boundary_x) * 0.5,
        ZONE_STRIP * 0.55,
    );

    // Lane rows.
    for lane in 0..LANES {
        let y = lane_top + lane as f64 * lane_h;
        ctx.set_stroke_style_str(LINE_SUBTLE);
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(0.0, y);
        ctx.line_to(LOGICAL_W, y);
        ctx.stroke();
        if lane == snapshot.selected_lane {
            ctx.set_fill_style_str("rgba(121,222,245,0.05)");
            ctx.fill_rect(0.0, y, LOGICAL_W, lane_h);
            ctx.set_fill_style_str(CYAN);
            ctx.fill_rect(0.0, y + 4.0, 5.0, lane_h - 8.0);
        }
    }

    // Membrane: strong cyan + inner gold trace.
    ctx.set_stroke_style_str(CYAN);
    ctx.set_line_width(3.0);
    ctx.begin_path();
    ctx.move_to(boundary_x - 2.0, lane_top);
    ctx.line_to(boundary_x - 2.0, LOGICAL_H);
    ctx.stroke();
    ctx.set_stroke_style_str(GOLD);
    ctx.set_line_width(1.0);
    ctx.begin_path();
    ctx.move_to(boundary_x + 4.0, lane_top);
    ctx.line_to(boundary_x + 4.0, LOGICAL_H);
    ctx.stroke();

    // Selected-lane gate bracket and cooldown trace.
    let sel_y = lane_top + snapshot.selected_lane as f64 * lane_h;
    ctx.set_stroke_style_str("#bfeefb");
    ctx.set_line_width(3.0);
    let bracket_x = boundary_x - 2.0;
    ctx.begin_path();
    ctx.move_to(bracket_x - 14.0, sel_y + 8.0);
    ctx.line_to(bracket_x, sel_y + 8.0);
    ctx.line_to(bracket_x, sel_y + 26.0);
    ctx.move_to(bracket_x - 14.0, sel_y + lane_h - 8.0);
    ctx.line_to(bracket_x, sel_y + lane_h - 8.0);
    ctx.line_to(bracket_x, sel_y + lane_h - 26.0);
    ctx.stroke();
    if snapshot.cooldown > 0 {
        let fraction = snapshot.cooldown as f64 / ACTION_COOLDOWN as f64;
        ctx.set_fill_style_str(GOLD);
        ctx.fill_rect(
            bracket_x - window_w,
            sel_y + lane_h - 6.0,
            window_w * fraction.min(1.0),
            3.0,
        );
    }

    // Entities.
    let radius = 17.0;
    let label_visible = field.scale >= 0.55;
    for entity in simulation.entities().iter().flatten() {
        let x = LOGICAL_W * entity.x as f64 / FIELD_END_X as f64;
        let y = lane_top + entity.lane as f64 * lane_h + lane_h * 0.5;
        let (stroke, fill) = palette(entity.kind);

        if !reduced_motion && entity.kind.risk_category() == RiskCategory::Claim {
            ctx.set_stroke_style_str("rgba(214,185,107,0.30)");
            ctx.set_line_width(2.0);
            ctx.begin_path();
            ctx.move_to(x - radius - 16.0, y);
            ctx.line_to(x - radius - 4.0, y);
            ctx.stroke();
        }

        ctx.set_fill_style_str(fill);
        ctx.begin_path();
        let _ = ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        ctx.fill();

        let dashed = entity.kind == EntityKind::UnknownPacket;
        if dashed {
            let dash = js_sys::Array::new();
            dash.push(&JsValue::from_f64(5.0));
            dash.push(&JsValue::from_f64(4.0));
            let _ = ctx.set_line_dash(dash.unchecked_ref());
        }
        ctx.set_stroke_style_str(stroke);
        ctx.set_line_width(2.5);
        ctx.begin_path();
        let _ = ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        ctx.stroke();
        if dashed {
            let empty = js_sys::Array::new();
            let _ = ctx.set_line_dash(empty.unchecked_ref());
        }
        if entity.kind == EntityKind::ValidatedIntent {
            ctx.begin_path();
            let _ = ctx.arc(x, y, radius - 5.0, 0.0, std::f64::consts::TAU);
            ctx.stroke();
        }

        ctx.set_font(&format!("700 16px {MONO}"));
        ctx.set_fill_style_str(if entity.kind == EntityKind::TypedIntent {
            "#06251d"
        } else {
            stroke
        });
        let _ = ctx.fill_text(entity.kind.symbol(), x, y + 1.0);

        if label_visible {
            ctx.set_font(&format!("600 11px {MONO}"));
            ctx.set_fill_style_str(TEXT_DIM);
            let _ = ctx.fill_text(entity.kind.label(), x, y + radius + 11.0);
        }
    }

    ctx.restore();
}

fn palette(kind: EntityKind) -> (&'static str, &'static str) {
    if kind == EntityKind::TypedIntent {
        return (SAFE, SAFE);
    }
    match kind.risk_category() {
        RiskCategory::Critical => (DANGER, "rgba(255,113,134,0.10)"),
        RiskCategory::Hazard => (DANGER, "rgba(255,113,134,0.06)"),
        RiskCategory::Claim => (GOLD, "rgba(214,185,107,0.08)"),
        RiskCategory::Unknown => (SPECIAL, "rgba(169,147,255,0.10)"),
        RiskCategory::Payload | RiskCategory::Resource => (CYAN, "rgba(121,222,245,0.07)"),
        RiskCategory::Sealed => (SAFE, SAFE),
    }
}
