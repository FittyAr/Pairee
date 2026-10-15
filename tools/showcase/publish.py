"""Turn the rendered screens into the website assets: WebP slides, thumbnails,
the social card and the brand art.

Usage: python publish.py <png-dir> <site-assets-dir> <pairee-repo>
"""

import os
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

src, out, repo = sys.argv[1:4]
NAMES = ["panels", "quickview", "editor", "git", "menu", "image",
         "copy", "archive", "find", "palette", "help", "spanish"]

screens = os.path.join(out, "screens")
os.makedirs(screens, exist_ok=True)
for i, n in enumerate(NAMES):
    im = Image.open(os.path.join(src, f"{i + 1:02d}-{n}.png")).convert("RGB")
    im.save(os.path.join(screens, f"{i + 1:02d}-{n}.webp"), "WEBP", quality=90, method=6)
    thumb = im.resize((320, round(320 * im.height / im.width)), Image.LANCZOS)
    thumb.save(os.path.join(screens, f"{i + 1:02d}-{n}-thumb.webp"), "WEBP", quality=82, method=6)

# Icons from the repository logo (manifests/msix/Assets/StoreLogo.png is the
# raster of img/pairee_logo.svg).
logo = Image.open(os.path.join(repo, "manifests", "msix", "Assets", "StoreLogo.png")).convert("RGBA")
for size in (32, 180):
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    scaled = logo.copy()
    scaled.thumbnail((size, size), Image.LANCZOS)
    canvas.paste(scaled, ((size - scaled.width) // 2, (size - scaled.height) // 2), scaled)
    canvas.save(os.path.join(out, f"icon-{size}.png"), optimize=True)

# Brand cards (the 2160 px PNGs are too heavy for the page).
for lang in ("EN", "ES"):
    path = os.path.join(out, f"Pairee_{lang}_2160x2160.png")
    if os.path.exists(path):
        card = Image.open(path).convert("RGB")
        card.thumbnail((900, 900), Image.LANCZOS)
        card.save(os.path.join(out, f"brand-{lang.lower()}.webp"), "WEBP", quality=84, method=6)

# Social card, 1200 x 630.
W, H = 1200, 630
font_path = r"C:\Windows\Fonts\CascadiaMono.ttf"


def font(size, weight="Bold"):
    f = ImageFont.truetype(font_path, size)
    try:
        f.set_variation_by_name(weight)
    except Exception:
        pass
    return f


bg = Image.new("RGB", (W, H), (6, 9, 18))
d = ImageDraw.Draw(bg)
for r in range(560, 0, -4):
    a = int(46 * (1 - r / 560))
    d.ellipse([W - 240 - r, -140 - r, W - 240 + r, -140 + r], fill=(6 + a // 3, 9 + a, 18 + a * 2))
shot = Image.open(os.path.join(src, "01-panels.png")).convert("RGB")
shot = shot.resize((720, round(720 * shot.height / shot.width)), Image.LANCZOS)
mask = Image.new("L", shot.size, 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, *shot.size], 12, fill=255)
shadow = Image.new("L", (W, H), 0)
ImageDraw.Draw(shadow).rounded_rectangle([540, 190, 540 + shot.width, 190 + shot.height], 12, fill=180)
bg.paste((0, 0, 0), mask=shadow.filter(ImageFilter.GaussianBlur(24)))
bg.paste(shot, (520, 165), mask)
icon = logo.copy()
icon.thumbnail((120, 120), Image.LANCZOS)
bg.paste(icon, (64, 70), icon)
d = ImageDraw.Draw(bg)
d.text((64, 200), "Pairee", font=font(72), fill=(243, 246, 252))
d.text((64, 292), "Your world,", font=font(38), fill=(120, 200, 255))
d.text((64, 340), "in two panels.", font=font(38), fill=(0, 220, 230))
d.text((64, 420), "Terminal file manager", font=font(24, "Regular"), fill=(150, 160, 180))
d.text((64, 456), "Rust · Windows · Linux · macOS", font=font(24, "Regular"), fill=(150, 160, 180))
bg.save(os.path.join(out, "social.png"), optimize=True)
print("published", len(NAMES), "screens")
