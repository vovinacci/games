"""Draws Bounty's icon and writes it as PNGs of every size the packages need, into icon/ beside this file.

    python3 packaging/make_icon.py

A placeholder made of the game's own marks: the hero's filled circle, a band's hollow square, the prize's diamond.
Standard library only, so that it runs anywhere; replace the PNGs by an artist's when there are any.
"""

import pathlib
import struct
import zlib

SIZES = (16, 32, 64, 128, 256, 512, 1024)
BASE = 1024
SAMPLES = 2

GROUND = (18, 24, 38)
EDGE = (76, 87, 107)
HERO = (115, 184, 255)
BAND = (255, 140, 89)
PRIZE = (255, 230, 77)


def colour_at(x, y):
    """The colour at a point of the unit square, or None outside the icon's rounded square."""
    radius = 0.18
    cx = min(max(x, radius), 1 - radius)
    cy = min(max(y, radius), 1 - radius)
    if (x - cx) ** 2 + (y - cy) ** 2 > radius**2:
        return None
    if (x - 0.30) ** 2 + (y - 0.32) ** 2 < 0.13**2:
        return HERO
    dx, dy = abs(x - 0.70), abs(y - 0.32)
    if max(dx, dy) < 0.13 and max(dx, dy) > 0.075:
        return BAND
    diamond = abs(x - 0.5) + abs(y - 0.68)
    if 0.12 < diamond < 0.19:
        return PRIZE
    border = min(x, y, 1 - x, 1 - y)
    if border < 0.035:
        return EDGE
    return GROUND


def base_image():
    """The icon at the largest size, as rows of (r, g, b, a) with the edges of every shape smoothed."""
    rows = []
    step = 1 / (BASE * SAMPLES)
    for py in range(BASE):
        row = []
        for px in range(BASE):
            total = [0, 0, 0, 0]
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    colour = colour_at((px * SAMPLES + sx + 0.5) * step, (py * SAMPLES + sy + 0.5) * step)
                    if colour is not None:
                        for i in range(3):
                            total[i] += colour[i]
                        total[3] += 255
            covered = total[3] // 255
            n = SAMPLES * SAMPLES
            row.append(tuple(total[i] // covered if covered else 0 for i in range(3)) + (total[3] // n,))
        rows.append(row)
    return rows


def shrink(rows, size):
    """The image at a smaller size: each pixel the mean of the block it stands for, weighted by how solid."""
    block = len(rows) // size
    out = []
    for y in range(size):
        line = []
        for x in range(size):
            red = green = blue = alpha = 0
            for by in range(block):
                source = rows[y * block + by]
                for bx in range(block):
                    r, g, b, a = source[x * block + bx]
                    red += r * a
                    green += g * a
                    blue += b * a
                    alpha += a
            if alpha:
                line.append((red // alpha, green // alpha, blue // alpha, alpha // (block * block)))
            else:
                line.append((0, 0, 0, 0))
        out.append(line)
    return out


def png(rows):
    """The bytes of a PNG file with these rows of (r, g, b, a)."""

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    size = len(rows)
    raw = b"".join(b"\x00" + bytes(value for pixel in row for value in pixel) for row in rows)
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def main():
    folder = pathlib.Path(__file__).parent / "icon"
    folder.mkdir(exist_ok=True)
    base = base_image()
    for size in SIZES:
        rows = base if size == BASE else shrink(base, size)
        (folder / f"bounty-{size}.png").write_bytes(png(rows))


if __name__ == "__main__":
    main()
