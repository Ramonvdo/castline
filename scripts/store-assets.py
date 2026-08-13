"""Generate the Microsoft Store artwork from the app icon and brand palette.

    python scripts/store-assets.py logos   # logos, poster/box/hero art
    python scripts/store-assets.py shots   # wrap raw screenshots in captioned frames
    python scripts/store-assets.py all

`logos` needs nothing but icon.png. `shots` reads raw captures from
store-assets/raw/ (named 1-library.png, 2-fill.png, …) and composes each onto a
branded 1920x1080 frame with a caption, so the listing looks like one set rather
than five loose screen grabs.

Everything lands in store-assets/ — upload from there, keep it out of the app.
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "store-assets"
RAW = OUT / "raw"

# Palette lifted from src/app.css so the art matches the app exactly.
BG = (28, 38, 50)  # --bg      #1c2632
ACCENT = (139, 159, 164)  # --accent  #8b9fa4
INK = (230, 236, 238)  # --text    #e6ecee
MUTED = (154, 171, 176)  # --muted   #9aabb0
LINE = (139, 159, 164)  # grid / borders, used at low alpha

FONTS = Path("C:/Windows/Fonts")


def font(size, weight="regular"):
    names = {"regular": "segoeui.ttf", "semibold": "seguisb.ttf", "bold": "segoeuib.ttf"}
    path = FONTS / names[weight]
    if path.exists():
        return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()  # keeps the script runnable off-Windows


def backdrop(w, h, grid=True):
    """Brand background: slate base, soft accent glow, faint blueprint grid."""
    img = Image.new("RGB", (w, h), BG)

    # Radial glow, brightest a little above centre — same feel as the website hero.
    glow = ImageOps.invert(Image.radial_gradient("L")).resize((w * 2, int(h * 1.6)))
    canvas = Image.new("L", (w, h), 0)
    canvas.paste(glow, (-w // 2, -int(h * 0.55)))
    img.paste(Image.new("RGB", (w, h), ACCENT), (0, 0), canvas.point(lambda v: int(v * 0.20)))

    if grid:
        step = max(28, w // 34)
        overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        d = ImageDraw.Draw(overlay)
        for x in range(0, w, step):
            d.line([(x, 0), (x, h)], fill=(*LINE, 16))
        for y in range(0, h, step):
            d.line([(0, y), (w, y)], fill=(*LINE, 16))
        img = Image.alpha_composite(img.convert("RGBA"), overlay).convert("RGB")

    return img


def place_icon(img, box_size, center):
    icon = Image.open(ROOT / "icon.png").convert("RGBA")
    icon = icon.resize((box_size, box_size), Image.LANCZOS)

    # Drop shadow so the tile reads as a raised object, not a sticker.
    shadow = Image.new("RGBA", img.size, (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    pad = int(box_size * 0.06)
    sd.rounded_rectangle(
        [center[0] - box_size // 2, center[1] - box_size // 2 + pad,
         center[0] + box_size // 2, center[1] + box_size // 2 + pad],
        radius=int(box_size * 0.22), fill=(0, 0, 0, 150),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(box_size * 0.06))
    img = Image.alpha_composite(img.convert("RGBA"), shadow)
    img.paste(icon, (center[0] - box_size // 2, center[1] - box_size // 2), icon)
    return img.convert("RGB")


def centered(draw, text, y, f, fill):
    w = draw.textbbox((0, 0), text, font=f)[2]
    draw.text(((draw.im.size[0] - w) // 2, y), text, font=f, fill=fill)


def art(w, h, with_text, path):
    img = backdrop(w, h)
    scale = min(w, h)
    if with_text:
        img = place_icon(img, int(scale * 0.34), (w // 2, int(h * 0.40)))
        d = ImageDraw.Draw(img)
        centered(d, "Castline", int(h * 0.60), font(int(scale * 0.095), "semibold"), INK)
        centered(d, "Cast it once. Paste it anywhere.", int(h * 0.60) + int(scale * 0.125),
                 font(int(scale * 0.038)), MUTED)
    else:
        # Hero art must not carry the product title — icon only.
        img = place_icon(img, int(h * 0.42), (w // 2, h // 2))
    img.save(path)
    print(f"  {path.name}  {w}x{h}")


def logos():
    OUT.mkdir(exist_ok=True)
    print("Store logos:")
    icon = Image.open(ROOT / "icon.png").convert("RGBA")
    for size in (300, 150, 71):
        p = OUT / f"logo-{size}.png"
        icon.resize((size, size), Image.LANCZOS).save(p)
        print(f"  {p.name}  {size}x{size}")

    print("Poster art (9:16):")
    art(720, 1080, True, OUT / "poster-720x1080.png")
    art(1440, 2160, True, OUT / "poster-1440x2160.png")
    print("Box art (1:1):")
    art(1080, 1080, True, OUT / "box-1080x1080.png")
    art(2160, 2160, True, OUT / "box-2160x2160.png")
    print("Super hero art (16:9, no title text):")
    art(1920, 1080, False, OUT / "hero-1920x1080.png")


# Caption per raw screenshot. The number prefix decides the order in the listing.
CAPTIONS = {
    "1-library": ("Every prompt, template and SOP in one place",
                  "Colour-coded folders. Search, tags, and the ones you use most, first."),
    "2-fill": ("Fill {{variables}} from a profile, copy in one click",
               "Live preview shows exactly what lands on your clipboard."),
    "3-blueprint": ("Share a template as a blueprint",
                    "Export a .json, send it to anyone, import in one step."),
    "4-quickfind": ("Ctrl+K. Type. Copied.",
                    "Find any template instantly, without leaving the keyboard."),
    "5-sop": ("Walk through multi-step SOPs",
              "Copy one step at a time, in order, without losing your place."),
}


def shots():
    if not RAW.exists() or not any(RAW.glob("*.png")):
        print(f"No raw screenshots found in {RAW}")
        print("Drop your captures there named 1-library.png, 2-fill.png, 3-blueprint.png,")
        print("4-quickfind.png, 5-sop.png — then re-run.")
        return

    W, H = 1920, 1080
    print("Screenshots:")
    for src in sorted(RAW.glob("*.png")):
        title, sub = CAPTIONS.get(src.stem, (src.stem.replace("-", " ").title(), ""))
        img = backdrop(W, H)
        d = ImageDraw.Draw(img)
        centered(d, title, 62, font(52, "semibold"), INK)
        if sub:
            centered(d, sub, 132, font(28), MUTED)

        # Fit the capture into the lower area, preserving aspect.
        shot = Image.open(src).convert("RGB")
        avail_w, avail_h = W - 220, H - 300
        ratio = min(avail_w / shot.width, avail_h / shot.height)
        shot = shot.resize((int(shot.width * ratio), int(shot.height * ratio)), Image.LANCZOS)
        x, y = (W - shot.width) // 2, 215

        # Rounded corners + border, matching the app's own card radius.
        mask = Image.new("L", shot.size, 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, shot.width, shot.height], radius=12, fill=255)
        shadow = Image.new("RGBA", img.size, (0, 0, 0, 0))
        ImageDraw.Draw(shadow).rounded_rectangle(
            [x, y + 14, x + shot.width, y + shot.height + 14], radius=12, fill=(0, 0, 0, 170))
        img = Image.alpha_composite(img.convert("RGBA"),
                                    shadow.filter(ImageFilter.GaussianBlur(22))).convert("RGB")
        img.paste(shot, (x, y), mask)
        ImageDraw.Draw(img).rounded_rectangle(
            [x, y, x + shot.width, y + shot.height], radius=12, outline=(*LINE, 255), width=1)

        out = OUT / f"screenshot-{src.stem}.png"
        img.save(out)
        print(f"  {out.name}  {W}x{H}   <- {src.name}")


if __name__ == "__main__":
    what = sys.argv[1] if len(sys.argv) > 1 else "all"
    OUT.mkdir(exist_ok=True)
    if what in ("logos", "all"):
        logos()
    if what in ("shots", "all"):
        shots()
