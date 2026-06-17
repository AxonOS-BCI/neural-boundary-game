#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Denis Yermakou
# SPDX-FileContributor: AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
"""Generate 1280x720 preview.png for Neural Boundary Game v7.3.0."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def main() -> int:
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError:
        print("Pillow not installed: pip install Pillow --break-system-packages"); return 1
    W, H = 1280, 720
    FIELD_H, ZONE_TOP = 480, 50
    LANE_H = (FIELD_H - ZONE_TOP) / 5
    BOUNDARY = int(704 / 1024 * W)
    img = Image.new("RGB", (W, H), (3, 5, 7))
    draw = ImageDraw.Draw(img)
    # gradient background
    for y in range(H):
        t = y / H
        draw.line([(0, y), (W-1, y)], fill=(int(3+5*t), int(5+8*t), int(7+13*t)))
    # header
    draw.rectangle([(0,0),(W,44)], fill=(8,13,18))
    try:
        mf = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 13)
        bf = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 15)
        sf = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 11)
    except Exception:
        mf = bf = sf = ImageFont.load_default()
    draw.text((16,14), "AXONOS", fill=(121,222,245), font=mf)
    draw.text((110,15), "STANDARD · STANDARD", fill=(150,160,175), font=sf)
    draw.text((20, ZONE_TOP//2-6), "SIGNAL ZONE", fill=(70,80,95), font=sf)
    draw.text((BOUNDARY-55, ZONE_TOP//2-6), "BOUNDARY", fill=(121,222,245), font=sf)
    draw.text((BOUNDARY+60, ZONE_TOP//2-6), "APPLICATION", fill=(120,230,173), font=sf)
    # zones
    draw.rectangle([(BOUNDARY-int(160/1024*W), ZONE_TOP),(BOUNDARY, FIELD_H+ZONE_TOP)], fill=(10,18,28))
    draw.rectangle([(BOUNDARY, ZONE_TOP),(W, FIELD_H+ZONE_TOP)], fill=(8,22,14))
    # lanes
    for i in range(6):
        y = ZONE_TOP + int(i * LANE_H)
        draw.line([(0,y),(W,y)], fill=(25,35,50))
    # selected lane
    sy = ZONE_TOP + int(2 * LANE_H)
    draw.rectangle([(0,sy),(W,sy+int(LANE_H))], fill=(10,18,26))
    draw.rectangle([(0,sy),(4,sy+int(LANE_H))], fill=(121,222,245))
    # membrane
    draw.rectangle([(BOUNDARY-2,ZONE_TOP),(BOUNDARY+1,FIELD_H+ZONE_TOP)], fill=(121,222,245))
    draw.rectangle([(BOUNDARY+4,ZONE_TOP),(BOUNDARY+5,FIELD_H+ZONE_TOP)], fill=(214,185,107))
    # gate dots
    gc = [(120,230,173)]*5 + [(255,113,134)]*2
    for g in range(7):
        gy = ZONE_TOP + int(FIELD_H*(g+1)/8)
        draw.ellipse([(BOUNDARY-6,gy-5),(BOUNDARY+4,gy+5)], fill=gc[g])
    # entities
    ents = [(0,.30,(255,113,134),"◉"),(1,.50,(121,222,245),"◇"),(2,.62,(120,230,173),"⬡"),
            (2,.73,(121,222,245),"◈"),(3,.38,(214,185,107),"△"),(3,.56,(169,147,255),"◌"),
            (4,.45,(120,230,173),"▣"),(4,.69,(255,113,134),"✕")]
    for lane,xp,col,sym in ents:
        ex,ey,r = int(xp*W), int(ZONE_TOP+lane*LANE_H+LANE_H/2), 18
        draw.ellipse([(ex-r,ey-r),(ex+r,ey+r)], fill=(col[0]//8,col[1]//8,col[2]//8), outline=col, width=2)
        draw.text((ex-7,ey-9), sym, fill=col, font=mf)
    # HUD strip
    hy = FIELD_H + ZONE_TOP + 8
    draw.rectangle([(0,hy-4),(W,hy+62)], fill=(8,13,18))
    metrics = [("TRUST","742",(121,222,245)),("RISK","183",(255,113,134)),("INTEGRITY","868",(120,230,173)),
               ("SCORE","14920",(244,241,232)),("COMBO","×7",(244,241,232)),("EVIDENCE","L2",(244,241,232)),
               ("CONSENT","ACTIVE",(120,230,173)),("LEAKS","0",(244,241,232)),("TIME","02:14",(244,241,232))]
    mw = (W-20) // len(metrics)
    for i,(k,v,vc) in enumerate(metrics):
        x = 10 + i*mw
        draw.rectangle([(x,hy),(x+mw-4,hy+54)], fill=(13,20,27), outline=(25,35,50))
        draw.text((x+6,hy+5), k, fill=(100,110,125), font=sf)
        draw.text((x+6,hy+20), v, fill=vc, font=mf)
    # deck
    dy = H-68
    deck = [("1","VALIDATE"),(  "2","CONVERT"),("3","QUARANTINE"),("4","CONSENT"),("5","EVIDENCE"),("⏎","RELEASE")]
    bw2 = (W-52)//6
    for i,(k,lb) in enumerate(deck):
        bx = 26+i*(bw2+5)
        bc = (214,185,107) if lb=="RELEASE" else (25,35,50)
        draw.rectangle([(bx,dy),(bx+bw2,H-4)], fill=(13,20,27), outline=bc)
        kc = (214,185,107) if lb=="RELEASE" else (121,222,245)
        draw.text((bx+6,dy+7), k, fill=kc, font=mf)
        draw.text((bx+4,dy+28), lb, fill=(130,140,155), font=sf)
    draw.text((W-260,H-18), "v7.3.0 · COGNITIVE SOVEREIGNTY", fill=(45,55,70), font=sf)
    out = ROOT / "preview.png"
    img.save(out, "PNG", optimize=True)
    print(f"preview.png: {W}×{H}, {out.stat().st_size:,} bytes")
    return 0

if __name__ == "__main__":
    sys.exit(main())
