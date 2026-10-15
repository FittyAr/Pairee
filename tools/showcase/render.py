"""Render the screen dumps of `capture` (JSON) to PNG, like a terminal would.

Usage: python render.py <json-dir> <png-dir> [--font PATH] [--size PT]

Box-drawing and block characters are drawn as shapes so lines join without
gaps; everything else uses the monospace font (Cascadia Mono by default,
SIL Open Font License, shipped with Windows 11 and Windows Terminal).
"""

import argparse
import json
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser()
parser.add_argument("src")
parser.add_argument("dst")
parser.add_argument("--font", default=None)
parser.add_argument("--size", type=int, default=17)
args = parser.parse_args()

FONT_CANDIDATES = [
    args.font,
    r"C:\Windows\Fonts\CascadiaMono.ttf",
    "/usr/share/fonts/truetype/cascadia-code/CascadiaMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
]
font_path = next(p for p in FONT_CANDIDATES if p and os.path.exists(p))
bold_path = font_path.replace("CascadiaMono.ttf", "CascadiaMono.ttf")
font = ImageFont.truetype(font_path, args.size)
try:
    font_bold = ImageFont.truetype(font_path, args.size)
    font_bold.set_variation_by_name("Bold")
except Exception:  # not a variable font
    font_bold = font
fallback = None
for p in [r"C:\Windows\Fonts\seguisym.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"]:
    if os.path.exists(p):
        fallback = ImageFont.truetype(p, args.size)
        break

ascent, descent = font.getmetrics()
CW = round(font.getlength("M"))
CH = round((ascent + descent) * 1.08)

# Windows Terminal "Campbell"-like palette for the 16 indexed colors.
BASE16 = [
    (12, 12, 12), (197, 15, 31), (19, 161, 14), (193, 156, 0), (0, 55, 218), (136, 23, 152), (58, 150, 221), (204, 204, 204),
    (118, 118, 118), (231, 72, 86), (22, 198, 12), (249, 241, 165), (59, 120, 255), (180, 0, 158), (97, 214, 214), (242, 242, 242),
]
DEFAULT_FG = (204, 204, 204)
DEFAULT_BG = (12, 12, 16)


def indexed(i):
    if i < 16:
        return BASE16[i]
    if i < 232:
        i -= 16
        steps = [0, 95, 135, 175, 215, 255]
        return steps[i // 36], steps[(i // 6) % 6], steps[i % 6]
    v = 8 + (i - 232) * 10
    return v, v, v


def color(value, default):
    if value is None:
        return default
    if isinstance(value, int):
        return indexed(value)
    return tuple(value)


# Box drawing: (up, right, down, left) weights; 1 light, 2 heavy, 3 double.
BOX = {}
def box(chars, *weights):
    for ch in chars:
        BOX[ch] = weights
box("─", 0, 1, 0, 1); box("━", 0, 2, 0, 2); box("═", 0, 3, 0, 3)
box("│", 1, 0, 1, 0); box("┃", 2, 0, 2, 0); box("║", 3, 0, 3, 0)
box("┌╭", 0, 1, 1, 0); box("┐╮", 0, 0, 1, 1); box("└╰", 1, 1, 0, 0); box("┘╯", 1, 0, 0, 1)
box("┏", 0, 2, 2, 0); box("┓", 0, 0, 2, 2); box("┗", 2, 2, 0, 0); box("┛", 2, 0, 0, 2)
box("╔", 0, 3, 3, 0); box("╗", 0, 0, 3, 3); box("╚", 3, 3, 0, 0); box("╝", 3, 0, 0, 3)
box("├", 1, 1, 1, 0); box("┤", 1, 0, 1, 1); box("┬", 0, 1, 1, 1); box("┴", 1, 1, 0, 1); box("┼", 1, 1, 1, 1)
box("┣", 2, 2, 2, 0); box("┫", 2, 0, 2, 2); box("┳", 0, 2, 2, 2); box("┻", 2, 2, 0, 2); box("╋", 2, 2, 2, 2)
box("╠", 3, 3, 3, 0); box("╣", 3, 0, 3, 3); box("╦", 0, 3, 3, 3); box("╩", 3, 3, 0, 3); box("╬", 3, 3, 3, 3)
box("╟", 3, 1, 3, 0); box("╢", 3, 0, 3, 1); box("╤", 0, 3, 1, 3); box("╧", 1, 3, 0, 3)
box("╞", 1, 3, 1, 0); box("╡", 1, 0, 1, 3); box("╥", 0, 1, 3, 1); box("╨", 3, 1, 0, 1)
box("╪", 1, 3, 1, 3); box("╫", 3, 1, 3, 1)
box("╒", 0, 3, 1, 0); box("╕", 0, 0, 1, 3); box("╘", 1, 3, 0, 0); box("╛", 1, 0, 0, 3)
box("╓", 0, 1, 3, 0); box("╖", 0, 0, 3, 1); box("╙", 3, 1, 0, 0); box("╜", 3, 0, 0, 1)
box("╴", 0, 0, 0, 1); box("╵", 1, 0, 0, 0); box("╶", 0, 1, 0, 0); box("╷", 0, 0, 1, 0)


def draw_box(d, x, y, weights, fg):
    up, right, down, left = weights
    cx, cy = x + CW // 2, y + CH // 2
    light = max(1, round(args.size / 14))
    heavy = light * 2 + 1
    gap = light + 1  # half the distance between the two strokes of a double line

    def stroke(x0, y0, x1, y1, w):
        if w == 1:
            d.rectangle([x0 - (light - 1) // 2 if x0 == x1 else x0, y0 - (light - 1) // 2 if y0 == y1 else y0,
                         x1 + light // 2 if x0 == x1 else x1, y1 + light // 2 if y0 == y1 else y1], fill=fg)
        elif w == 2:
            h = heavy // 2
            d.rectangle([x0 - h if x0 == x1 else x0, y0 - h if y0 == y1 else y0, x1 + h if x0 == x1 else x1, y1 + h if y0 == y1 else y1], fill=fg)

    # Single and heavy arms.
    if up in (1, 2): stroke(cx, y, cx, cy, up)
    if down in (1, 2): stroke(cx, cy, cx, y + CH - 1, down)
    if left in (1, 2): stroke(x, cy, cx, cy, left)
    if right in (1, 2): stroke(cx, cy, x + CW - 1, cy, right)

    def vline(xx, y0, y1):
        d.rectangle([xx, min(y0, y1), xx + light - 1, max(y0, y1)], fill=fg)

    def hline(yy, x0, x1):
        d.rectangle([min(x0, x1), yy, max(x0, x1), yy + light - 1], fill=fg)

    top, bottom, lft, rgt = y, y + CH - 1, x, x + CW - 1
    dv = [s for s, w in ((-1, up), (1, down)) if w == 3]
    dh = [s for s, w in ((-1, left), (1, right)) if w == 3]
    if len(dv) == 1 and len(dh) == 1:
        # A double corner: the inner pair starts at the inside of the turn.
        sv, sh = dv[0], dh[0]
        vend = bottom if sv > 0 else top
        hend = rgt if sh > 0 else lft
        vline(cx + sh * gap, cy + sv * gap, vend)
        vline(cx - sh * gap, cy - sv * gap, vend)
        hline(cy + sv * gap, cx + sh * gap, hend)
        hline(cy - sv * gap, cx - sh * gap, hend)
        return
    for sv in dv:
        end = bottom if sv > 0 else top
        start = top if len(dv) == 2 else cy - sv * gap
        for off in (-gap, gap):
            vline(cx + off, start if len(dv) == 1 else top, end if len(dv) == 1 else bottom)
    for sh in dh:
        end = rgt if sh > 0 else lft
        start = cx - sh * gap
        for off in (-gap, gap):
            hline(cy + off, start if len(dh) == 1 else lft, end if len(dh) == 1 else rgt)


def draw_block(img, d, x, y, ch, fg, bg):
    o = ord(ch)
    if ch == "█":
        d.rectangle([x, y, x + CW - 1, y + CH - 1], fill=fg)
    elif ch == "▀":
        d.rectangle([x, y, x + CW - 1, y + CH // 2 - 1], fill=fg)
    elif 0x2581 <= o <= 0x2587:  # lower eighths
        h = round(CH * (o - 0x2580) / 8)
        d.rectangle([x, y + CH - h, x + CW - 1, y + CH - 1], fill=fg)
    elif ch == "▄":
        d.rectangle([x, y + CH // 2, x + CW - 1, y + CH - 1], fill=fg)
    elif 0x2589 <= o <= 0x258F:  # left eighths
        w = round(CW * (0x2590 - o) / 8)
        d.rectangle([x, y, x + w - 1, y + CH - 1], fill=fg)
    elif ch == "▌":
        d.rectangle([x, y, x + CW // 2 - 1, y + CH - 1], fill=fg)
    elif ch == "▐":
        d.rectangle([x + CW // 2, y, x + CW - 1, y + CH - 1], fill=fg)
    elif ch in "░▒▓":
        a = {"░": 0.25, "▒": 0.5, "▓": 0.75}[ch]
        mix = tuple(int(bg[i] + (fg[i] - bg[i]) * a) for i in range(3))
        d.rectangle([x, y, x + CW - 1, y + CH - 1], fill=mix)
    elif ch == "▔":
        d.rectangle([x, y, x + CW - 1, y + max(1, CH // 8) - 1], fill=fg)
    else:
        return False
    return True


def has_glyph(f, ch):
    try:
        return f.getmask(ch).getbbox() is not None or ch.isspace()
    except Exception:
        return False


def render(src, dst):
    doc = json.loads(Path(src).read_text(encoding="utf-8"))
    rows, cols = doc["rows"], doc["cols"]
    pad = 0
    img = Image.new("RGB", (cols * CW + 2 * pad, rows * CH + 2 * pad), DEFAULT_BG)
    d = ImageDraw.Draw(img)
    for r, line in enumerate(doc["cells"]):
        for c, cell in enumerate(line):
            if cell["wc"]:
                continue
            fg = color(cell["fg"], DEFAULT_FG)
            bg = color(cell["bg"], DEFAULT_BG)
            if cell["r"]:
                fg, bg = bg, fg
            x, y = pad + c * CW, pad + r * CH
            width = CW * (2 if cell["w"] else 1)
            d.rectangle([x, y, x + width - 1, y + CH - 1], fill=bg)
            ch = cell["t"]
            if not ch or ch == " ":
                continue
            if len(ch) == 1 and ch in BOX:
                draw_box(d, x, y, BOX[ch], fg)
                continue
            if len(ch) == 1 and 0x2580 <= ord(ch) <= 0x259F and draw_block(img, d, x, y, ch, fg, bg):
                continue
            f = font_bold if cell["b"] else font
            if fallback and not has_glyph(f, ch[0]):
                f = fallback
            d.text((x, y + (CH - ascent - descent) // 2), ch, font=f, fill=fg)
            if cell["u"]:
                d.line([(x, y + CH - 2), (x + width - 1, y + CH - 2)], fill=fg)
    img.save(dst)


out = Path(args.dst)
out.mkdir(parents=True, exist_ok=True)
for src in sorted(Path(args.src).glob("*.json")):
    render(src, out / (src.stem + ".png"))
    print("rendered", src.stem)
print(f"cell {CW}x{CH}px, font {font_path}")
