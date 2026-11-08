#!/usr/bin/env python3
"""Draw the Photo Resizer app icon and export PNG/ICO sizes.

The icon is a stylised photo card on a deep navy-to-indigo gradient with a
resize badge in the corner. It is rendered at 4x and downsampled with Lanczos
so the edges stay crisp at tray and taskbar sizes.

Usage:
    py tools/make_icon.py
    py tools/make_icon.py --output-dir assets/icons

Requires Pillow:
    py -m pip install pillow
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

# Render at 4x, then downsample once at the end.
SCALE = 4
SIZE = 1024
CANVAS = SIZE * SCALE

NAVY = (10, 14, 24, 255)
INDIGO = (43, 58, 143, 255)
ACCENT = (96, 130, 255, 255)
ACCENT_LIGHT = (136, 160, 255, 255)
CARD = (247, 249, 255, 255)
SKY_TOP = (128, 178, 255, 255)
SKY_BOTTOM = (219, 233, 255, 255)
SUN = (255, 196, 77, 255)
MOUNTAIN_BACK = (45, 74, 138, 255)
MOUNTAIN_FRONT = (79, 116, 216, 255)
WHITE = (255, 255, 255, 255)


def mix(a: tuple[int, int, int, int], b: tuple[int, int, int, int], t: float):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def vertical_gradient(width: int, height: int, top, bottom) -> Image.Image:
    strip = Image.new("RGBA", (1, height))
    pixels = strip.load()
    for y in range(height):
        pixels[0, y] = mix(top, bottom, y / max(height - 1, 1))
    return strip.resize((width, height), Image.Resampling.BILINEAR)


def rounded_mask(width: int, height: int, radius: int) -> Image.Image:
    mask = Image.new("L", (width, height), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, width - 1, height - 1), radius=radius, fill=255
    )
    return mask


def draw_arrowhead(draw: ImageDraw.ImageDraw, tip, tail, width: int, color):
    """Draw a filled triangular arrowhead at `tip` pointing away from `tail`."""
    dx = tip[0] - tail[0]
    dy = tip[1] - tail[1]
    length = (dx * dx + dy * dy) ** 0.5
    ux, uy = dx / length, dy / length
    px, py = -uy, ux

    base = (tip[0] - ux * width * 1.8, tip[1] - uy * width * 1.8)
    left = (base[0] + px * width * 0.9, base[1] + py * width * 0.9)
    right = (base[0] - px * width * 0.9, base[1] - py * width * 0.9)
    draw.polygon([tip, left, right], fill=color)


def draw_icon() -> Image.Image:
    background = vertical_gradient(CANVAS, CANVAS, NAVY, INDIGO)
    mask = rounded_mask(CANVAS, CANVAS, radius=round(CANVAS * 0.225))
    icon = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    icon.paste(background, (0, 0), mask)

    # Soft glow behind the photo card.
    glow = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow)
    glow_draw.ellipse(
        (CANVAS * 0.12, CANVAS * 0.1, CANVAS * 0.9, CANVAS * 0.92),
        fill=(96, 130, 255, 110),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(CANVAS * 0.09))
    icon = Image.alpha_composite(icon, glow)

    # Drop shadow under the card.
    shadow = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow)
    card_box = (
        round(CANVAS * 0.225),
        round(CANVAS * 0.195),
        round(CANVAS * 0.775),
        round(CANVAS * 0.735),
    )
    card_radius = round(CANVAS * 0.055)
    shadow_draw.rounded_rectangle(
        (
            card_box[0] + CANVAS * 0.015,
            card_box[1] + CANVAS * 0.025,
            card_box[2] + CANVAS * 0.015,
            card_box[3] + CANVAS * 0.025,
        ),
        radius=card_radius,
        fill=(0, 0, 0, 150),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(CANVAS * 0.025))
    icon = Image.alpha_composite(icon, shadow)

    # The photo card.
    card = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    card_draw = ImageDraw.Draw(card)
    card_draw.rounded_rectangle(
        card_box, radius=card_radius, fill=CARD
    )

    # Photo inside the white card.
    inset = round(CANVAS * 0.04)
    photo_box = (
        card_box[0] + inset,
        card_box[1] + inset,
        card_box[2] - inset,
        card_box[3] - inset,
    )
    photo_size = (photo_box[2] - photo_box[0], photo_box[3] - photo_box[1])
    photo = vertical_gradient(photo_size[0], photo_size[1], SKY_TOP, SKY_BOTTOM)
    photo_mask = rounded_mask(photo_size[0], photo_size[1], radius=round(card_radius * 0.6))
    photo_layer = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    photo_layer.paste(photo, (photo_box[0], photo_box[1]), photo_mask)

    p = ImageDraw.Draw(photo_layer)
    left, top, right, bottom = photo_box
    width = right - left
    height = bottom - top

    # Sun.
    sun_r = width * 0.095
    sun_cx = left + width * 0.72
    sun_cy = top + height * 0.25
    p.ellipse(
        (sun_cx - sun_r, sun_cy - sun_r, sun_cx + sun_r, sun_cy + sun_r),
        fill=SUN,
    )

    # Layered mountains.
    p.polygon(
        [
            (left, top + height * 0.78),
            (left + width * 0.27, top + height * 0.42),
            (left + width * 0.47, top + height * 0.66),
            (left + width * 0.63, top + height * 0.38),
            (right, top + height * 0.76),
            (right, bottom),
            (left, bottom),
        ],
        fill=MOUNTAIN_BACK,
    )
    p.polygon(
        [
            (left, top + height * 0.88),
            (left + width * 0.36, top + height * 0.55),
            (left + width * 0.58, top + height * 0.76),
            (left + width * 0.8, top + height * 0.52),
            (right, top + height * 0.86),
            (right, bottom),
            (left, bottom),
        ],
        fill=MOUNTAIN_FRONT,
    )
    icon = Image.alpha_composite(icon, card)
    icon = Image.alpha_composite(icon, photo_layer)

    # Resize badge: a circular accent button with a diagonal double arrow.
    badge_r = CANVAS * 0.135
    badge_cx = card_box[2] - CANVAS * 0.005
    badge_cy = card_box[3] - CANVAS * 0.005
    badge = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    badge_draw = ImageDraw.Draw(badge)
    badge_draw.ellipse(
        (
            badge_cx - badge_r,
            badge_cy - badge_r,
            badge_cx + badge_r,
            badge_cy + badge_r,
        ),
        fill=ACCENT,
        outline=WHITE,
        width=round(CANVAS * 0.012),
    )

    arrow_start = (badge_cx - badge_r * 0.52, badge_cy + badge_r * 0.52)
    arrow_end = (badge_cx + badge_r * 0.52, badge_cy - badge_r * 0.52)
    arrow_width = round(badge_r * 0.17)
    badge_draw.line([arrow_start, arrow_end], fill=WHITE, width=arrow_width)
    draw_arrowhead(badge_draw, arrow_end, arrow_start, arrow_width, WHITE)
    draw_arrowhead(badge_draw, arrow_start, arrow_end, arrow_width, WHITE)

    icon = Image.alpha_composite(icon, badge)
    return icon.resize((SIZE, SIZE), Image.Resampling.LANCZOS)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "assets" / "icons",
        help="Where to write the generated icons (default: assets/icons)",
    )
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)

    icon = draw_icon()

    icon.save(args.output_dir / "icon.png")
    for edge in (512, 256, 128, 64, 32):
        icon.resize((edge, edge), Image.Resampling.LANCZOS).save(
            args.output_dir / f"icon-{edge}.png"
        )

    icon.save(
        args.output_dir / "icon.ico",
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    print(f"Wrote icons to {args.output_dir}")


if __name__ == "__main__":
    main()

