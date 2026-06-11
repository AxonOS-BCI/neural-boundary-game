#!/usr/bin/env python3
"""Render preview.png (1280x720) for Neural Boundary Game v2.1.2.

Draws the Foundation Grande stage at 2x supersampling and downsamples with
Lanczos for crisp 1px-equivalent strokes. The scene is a believable mid-run
moment: consent open, three gates passed, mixed traffic at the membrane.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "preview.png"

S = 2  # supersample factor
W, H = 1280 * S, 720 * S

BG = "#03070B"
PANEL = "#071018"
PANEL2 = "#0B1520"
BORDER = "#143140"
CYAN = "#20D6FF"
CYAN_SOFT = "#8EEBFF"
GREEN = "#7CFF4F"
RED = "#FF4D4D"
VIOLET = "#9B7CFF"
AMBER = "#FF9F1C"
TEXT = "#EDF5FF"
MUTED = "#8EA0B6"
FAINT = "#617186"

FONT_DIR = Path("/usr/share/fonts/truetype/dejavu")


def font(name: str, size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONT_DIR / name), size * S)


SANS_B = lambda s: font("DejaVuSans-Bold.ttf", s)
SANS = lambda s: font("DejaVuSans.ttf", s)
MONO = lambda s: font("DejaVuSansMono.ttf", s)
MONO_B = lambda s: font("DejaVuSansMono-Bold.ttf", s)


def rr(draw: ImageDraw.ImageDraw, box, radius, fill=None, outline=None, width=1):
    draw.rounded_rectangle(
        [c * S for c in box], radius=radius * S, fill=fill, outline=outline, width=width * S
    )


def text(draw, xy, value, fnt, fill, anchor="la", tracking=0):
    if tracking:
        x, y = xy
        for ch in value:
            draw.text((x * S, y * S), ch, font=fnt, fill=fill, anchor=anchor)
            x += (fnt.getlength(ch) / S) + tracking
    else:
        draw.text((xy[0] * S, xy[1] * S), value, font=fnt, fill=fill, anchor=anchor)


def line(draw, a, b, fill, width=1):
    draw.line([a[0] * S, a[1] * S, b[0] * S, b[1] * S], fill=fill, width=width * S)


def main() -> None:
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)

    # Ambient gradient wash (vertical) + faint top glow.
    for y in range(720):
        t = y / 720
        r = int(3 + 4 * (1 - abs(t - 0.5) * 2) * 0.6)
        g = int(7 + 7 * (1 - abs(t - 0.55) * 2) * 0.7)
        b = int(11 + 10 * (1 - abs(t - 0.55) * 2) * 0.7)
        draw.rectangle([0, y * S, W, (y + 1) * S], fill=(r, g, b))

    # Stage frame.
    rr(draw, (8, 8, 1272, 712), 18, fill=None, outline=BORDER, width=1)

    pad = 24 + 8
    x0, y0 = pad, pad
    inner_w = 1280 - 2 * pad

    # ---------- header ----------
    text(draw, (x0, y0 + 2), "NEURAL BOUNDARY ", SANS_B(24), TEXT, tracking=3)
    tw = SANS_B(24).getlength("NEURAL BOUNDARY ") / S + 3 * len("NEURAL BOUNDARY ")
    text(draw, (x0 + tw, y0 + 2), "GAME", SANS_B(24), CYAN, tracking=3)
    text(draw, (x0, y0 + 42), "Do not ship raw signal. ", MONO(12), MUTED)
    off = MONO(12).getlength("Do not ship raw signal. ") / S
    text(draw, (x0 + off, y0 + 42), "Ship typed intent.", MONO_B(12), CYAN_SOFT)

    chips = [("AxonOS-BCI", CYAN), ("RUST → WASM", MUTED), ("no_std CORE", MUTED)]
    cx = x0 + 480
    for label, color in chips:
        cw = MONO(10).getlength(label) / S + 22
        rr(draw, (cx, y0 + 6, cx + cw, y0 + 26), 10, fill=PANEL2, outline=BORDER, width=1)
        text(draw, (cx + cw / 2, y0 + 16), label, MONO(10), color, anchor="mm")
        cx += cw + 8

    for i, label in enumerate(["H  HELP", "R  RESTART"]):
        bx = x0 + inner_w - 230 + i * 120
        rr(draw, (bx, y0 + 4, bx + 108, y0 + 36), 10, fill=PANEL2, outline=BORDER, width=1)
        text(draw, (bx + 54, y0 + 20), label, MONO(11), CYAN_SOFT, anchor="mm")

    # ---------- status rail ----------
    ry = y0 + 72 + 12
    cells = [
        ("RUN", "03", None, None),
        ("TRUST · TARGET ≥90", "76", CYAN, 0.76),
        ("RISK · LIMIT ≤20", "12", RED, 0.12),
        ("INTEGRITY · TARGET ≥80", "94", GREEN, 0.94),
        ("EVIDENCE · TARGET L2", "L2 · 5 pts", None, None),
        ("CONSENT", "OPEN · 18s", None, None),
    ]
    widths = [0.7, 1.2, 1.2, 1.2, 1.0, 1.0]
    total = sum(widths)
    cx = x0
    for (k, v, bar_color, frac), wgt in zip(cells, widths):
        cw = (inner_w - 10 * 5) * wgt / total
        rr(draw, (cx, ry, cx + cw, ry + 54), 12, fill=PANEL, outline=BORDER, width=1)
        text(draw, (cx + 12, ry + 9), k, MONO(8), FAINT, tracking=1)
        vx = cx + 12
        if k == "CONSENT":
            draw.ellipse(
                [(vx) * S, (ry + 26) * S, (vx + 8) * S, (ry + 34) * S], fill=GREEN
            )
            vx += 14
        text(draw, (vx, ry + 23), v, MONO_B(14), TEXT)
        if bar_color:
            bw = cw - 24
            rr(draw, (cx + 12, ry + 45, cx + 12 + bw, ry + 49), 2, fill="#142C3A")
            rr(draw, (cx + 12, ry + 45, cx + 12 + bw * frac, ry + 49), 2, fill=bar_color)
        cx += cw + 10

    # ---------- playfield ----------
    fy = ry + 54 + 12
    fw, fh = 916, 372
    rr(draw, (x0, fy, x0 + fw, fy + fh), 14, fill=BG, outline=BORDER, width=1)

    boundary = x0 + fw * 0.68
    window_w = fw * 0.32
    lane_top = fy + 24
    lane_h = (fh - 24) / 5

    # zone strip
    rr(draw, (x0, fy, x0 + fw, fy + 24), 8, fill=PANEL2, outline=BORDER, width=1)
    text(draw, (x0 + fw * 0.34, fy + 12), "SIGNAL ZONE", MONO(9), MUTED, anchor="mm", tracking=1)
    text(draw, (boundary, fy + 12), "BOUNDARY", MONO_B(9), CYAN, anchor="mm", tracking=1)
    text(
        draw,
        (boundary + (x0 + fw - boundary) / 2, fy + 12),
        "APP ZONE",
        MONO(9),
        GREEN,
        anchor="mm",
        tracking=1,
    )

    # zone tints
    draw.rectangle(
        [(boundary - window_w) * S, lane_top * S, boundary * S, (fy + fh - 1) * S],
        fill=(8, 22, 30),
    )
    draw.rectangle(
        [boundary * S, lane_top * S, (x0 + fw - 1) * S, (fy + fh - 1) * S], fill=(6, 15, 14)
    )

    selected = 2
    for lane in range(5):
        ly = lane_top + lane * lane_h
        line(draw, (x0 + 1, ly), (x0 + fw - 1, ly), "#0E2230", 1)
        if lane == selected:
            draw.rectangle(
                [(x0 + 1) * S, ly * S, (x0 + fw - 1) * S, (ly + lane_h) * S],
                fill=(8, 25, 34),
            )
            draw.rectangle([(x0 + 1) * S, (ly + 3) * S, (x0 + 4) * S, (ly + lane_h - 3) * S], fill=CYAN)

    # membrane
    line(draw, (boundary - 2, lane_top), (boundary - 2, fy + fh - 1), CYAN, 2)
    line(draw, (boundary + 3, lane_top), (boundary + 3, fy + fh - 1), "#0F4B5C", 1)
    line(draw, (x0 + fw - 2, lane_top), (x0 + fw - 2, fy + fh - 1), "#1E4D2A", 1)

    # gate bracket on selected lane
    sy = lane_top + selected * lane_h
    bx = boundary - 2
    for ya, yb in [(sy + 5, sy + 15), (sy + lane_h - 5, sy + lane_h - 15)]:
        line(draw, (bx - 8, ya), (bx, ya), CYAN_SOFT, 2)
        line(draw, (bx, ya), (bx, yb), CYAN_SOFT, 2)

    def card(cx_pos, lane, label, border, label_color, fill, underline=False):
        ly = lane_top + lane * lane_h + lane_h / 2
        left, top = cx_pos - 42, ly - 13
        rr(draw, (left, top, left + 84, top + 26), 6, fill=fill, outline=border, width=1)
        text(draw, (cx_pos, ly), label, MONO_B(10), label_color, anchor="mm")
        if underline:
            draw.rectangle(
                [(left + 5) * S, (top + 22) * S, (left + 79) * S, (top + 24) * S], fill=GREEN
            )

    GOODF = "#0B1520"
    card(x0 + 120, 0, "EVIDENCE", CYAN, CYAN_SOFT, GOODF)
    card(x0 + 305, 1, "RAW", RED, "#FF8585", "#190D12")
    card(x0 + 462, 2, "INTENT", CYAN, CYAN_SOFT, GOODF, underline=True)
    card(x0 + 233, 2, "CONSENT", CYAN, CYAN_SOFT, GOODF)
    card(x0 + 538, 3, "?PKT", VIOLET, "#C9B8FF", "#150F22")
    card(x0 + 360, 4, "CLAIM", AMBER, "#FFC46B", "#1A130A")
    card(boundary + 95, 1, "TYPED", GREEN, GREEN, "#0A1A10")
    card(boundary + 196, 3, "TYPED", GREEN, GREEN, "#0A1A10")
    card(x0 + 96, 3, "CHECKSUM", CYAN, CYAN_SOFT, GOODF)

    # cooldown sliver under bracket
    draw.rectangle(
        [(bx - window_w) * S, (sy + lane_h - 4) * S, (bx - window_w * 0.62) * S, (sy + lane_h - 2) * S],
        fill=AMBER,
    )

    # ---------- event strip ----------
    ey = fy + fh + 8
    rr(draw, (x0, ey, x0 + fw, ey + 34), 10, fill=PANEL, outline=BORDER, width=1)
    msgs = [
        ("TYPED INTENT DELIVERED · TRUST +6", CYAN_SOFT),
        ("REVIEW GATE PASSED · EVIDENCE", "#5E8FA3"),
        ("?PKT CLASSIFIED · RAW · RISK +4", "#7A6A4A"),
    ]
    mx = x0 + 14
    for value, color in msgs:
        text(draw, (mx, ey + 11), value, MONO(10), color)
        mx += MONO(10).getlength(value) / S + 22

    # ---------- right panel ----------
    px = x0 + fw + 16
    pw = inner_w - fw - 16
    ph = fh + 8 + 34
    rr(draw, (px, fy, px + pw, fy + ph), 14, fill=PANEL, outline=BORDER, width=1)
    ty = fy + 16
    text(draw, (px + 16, ty), "BOUNDARY PRINCIPLE", MONO_B(10), CYAN, tracking=2)
    ty += 26
    text(draw, (px + 16, ty), "Raw signal stays inside", SANS(13), TEXT)
    text(draw, (px + 16, ty + 19), "the device. Applications", SANS(13), TEXT)
    text(draw, (px + 16, ty + 38), "receive typed intent only.", SANS(13), TEXT)
    ty += 70
    text(draw, (px + 16, ty), "APPS RECEIVE", MONO(8), FAINT, tracking=1)
    text(draw, (px + 16, ty + 14), "typed intent · consent state", MONO(10), GREEN)
    text(draw, (px + 16, ty + 28), "· confidence", MONO(10), GREEN)
    ty += 54
    text(draw, (px + 16, ty), "APPS NEVER RECEIVE", MONO(8), FAINT, tracking=1)
    text(draw, (px + 16, ty + 14), "raw frames · spectral bands", MONO(10), RED)
    text(draw, (px + 16, ty + 28), "· waveforms", MONO(10), RED)
    ty += 54
    text(draw, (px + 16, ty), "RELEASE REQUIRES", MONO(8), FAINT, tracking=1)
    for i, req in enumerate(["TRUST ≥90 · RISK ≤20", "INTEGRITY ≥80 · EVIDENCE ≥L2", "5 GATES · 0 RAW LEAKS"]):
        text(draw, (px + 16, ty + 14 + i * 14), req, MONO(9), MUTED)

    gy = fy + ph - 64
    line(draw, (px + 16, gy), (px + pw - 16, gy), BORDER, 1)
    text(draw, (px + 16, gy + 10), "GATES PASSED 3/5", MONO(10), MUTED, tracking=1)
    gates = [("SCHEMA", True), ("CONSENT", True), ("EVIDENCE", True), ("CONTAIN", False), ("DELIVERY", False)]
    gw = (pw - 32 - 4 * 6) / 5
    gx = px + 16
    for label, passed in gates:
        if passed:
            rr(draw, (gx, gy + 28, gx + gw, gy + 46), 6, fill=CYAN)
            text(draw, (gx + gw / 2, gy + 37), label, MONO_B(7), "#04222C", anchor="mm")
        else:
            rr(draw, (gx, gy + 28, gx + gw, gy + 46), 6, fill=PANEL2, outline=BORDER, width=1)
            text(draw, (gx + gw / 2, gy + 37), label, MONO(7), FAINT, anchor="mm")
        gx += gw + 6

    # ---------- action bar ----------
    ay = ey + 34 + 12
    acts = [
        ("1", "VALIDATE", False),
        ("2", "CONVERT", False),
        ("3", "QUARANTINE", False),
        ("4", "CONSENT", False),
        ("5", "EVIDENCE", False),
        ("↵", "RELEASE", True),
    ]
    weights = [1, 1, 1, 1, 1, 1.25]
    total_w = sum(weights)
    ax = x0
    for (key, label, is_release), wgt in zip(acts, weights):
        aw = (inner_w - 10 * 5) * wgt / total_w
        border_c = "#5C3D14" if is_release else BORDER
        rr(draw, (ax, ay, ax + aw, ay + 54), 12, fill=PANEL, outline=border_c, width=1)
        key_color = AMBER if is_release else CYAN
        kw = MONO(10).getlength(key) / S + 14
        rr(draw, (ax + 16, ay + 18, ax + 16 + kw, ay + 36), 5, outline=key_color, width=1)
        text(draw, (ax + 16 + kw / 2, ay + 26.5), key, MONO(10), key_color, anchor="mm")
        text(
            draw,
            (ax + 16 + kw + 12, ay + 27),
            label,
            MONO(11),
            AMBER if is_release else TEXT,
            anchor="lm",
            tracking=1,
        )
        ax += aw + 10

    # ---------- footer ----------
    fy2 = ay + 54 + 12
    text(
        draw,
        (x0, fy2 + 8),
        "v2.1.2 • AxonOS Standard Foundation Grande Style • deterministic Rust core",
        MONO(10),
        FAINT,
        tracking=0.5,
    )
    right = "GITHUB      SEED 0x5A3C9E12"
    text(
        draw,
        (x0 + inner_w - MONO(10).getlength(right) / S, fy2 + 8),
        right,
        MONO(10),
        MUTED,
    )

    final = img.resize((1280, 720), Image.LANCZOS)
    final.save(OUT, optimize=True)
    print(f"wrote {OUT.relative_to(ROOT)} ({OUT.stat().st_size} bytes, 1280x720)")


if __name__ == "__main__":
    main()
