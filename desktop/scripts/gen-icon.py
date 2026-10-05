#!/usr/bin/env python3
"""Generate a simple AI Task Notify app icon (1024x1024 RGBA PNG).

Pure stdlib (no Pillow / no image decoding): draws a notification bell on a
rounded indigo square using per-pixel math with 4x supersampling. Intended to
be replaced by a real design later; exists so `tauri build` has an icon.
"""
import struct
import sys
import zlib

SIZE = 1024
SS = 4  # supersample factor
W = SIZE * SS

BG = (79, 70, 229)  # indigo  #4F46E5
BELL = (255, 255, 255)  # white
CLAP = (254, 240, 138)  # yellow #FEF08A
DOT = (52, 211, 153)  # green  #34D399


def in_rounded_square(x, y):
    m = 40
    r = 200
    x0, y0, x1, y1 = m, m, SIZE - m, SIZE - m
    if x < x0 or x > x1 or y < y0 or y > y1:
        return False
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    dx, dy = x - cx, y - cy
    return dx * dx + dy * dy <= r * r


def in_bell(x, y):
    # top dome (upper half of a circle)
    dx, dy = x - 512, y - 400
    if y <= 400 and dx * dx + dy * dy <= 250 * 250:
        return True
    # body: half-width narrows from 250 (top) to 160 (bottom)
    if 400 <= y <= 780:
        t = (y - 400) / 380.0
        hw = 250 - 90 * t
        if abs(x - 512) <= hw:
            return True
    # rounded bottom cap
    dx, dy = x - 512, y - 780
    if dx * dx + dy * dy <= 160 * 160:
        return True
    return False


def in_clapper(x, y):
    # connector
    if 780 <= y <= 880 and abs(x - 512) <= 26:
        return True
    dx, dy = x - 512, y - 900
    return dx * dx + dy * dy <= 72 * 72


def in_status_dot(x, y):
    dx, dy = x - 812, y - 248
    return dx * dx + dy * dy <= 88 * 88


def sample(x, y):
    """Color for one supersampled sub-pixel."""
    if in_status_dot(x, y):
        return DOT
    if in_clapper(x, y):
        return CLAP
    if in_bell(x, y):
        return BELL
    if in_rounded_square(x, y):
        return BG
    return (255, 255, 255)  # transparent-ish white; alpha set below


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "icon-src.png"
    rows = []
    for py in range(SIZE):
        row = []
        for px in range(SIZE):
            r = g = b = a = 0
            for sy in range(SS):
                for sx in range(SS):
                    x = px * SS + sx + 0.5
                    y = py * SS + sy + 0.5
                    if not in_rounded_square(x, y):
                        continue  # fully transparent
                    cr, cg, cb = sample(x, y)
                    r += cr
                    g += cg
                    b += cb
                    a += 255
            n = SS * SS
            if a == 0:
                row.append((0, 0, 0, 0))
            else:
                row.append((r // n, g // n, b // n, a // n))
        rows.append(row)

    def chunk(tag, data):
        payload = tag + data
        return (
            struct.pack(">I", len(data))
            + payload
            + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)
        )

    raw = bytearray()
    for row in rows:
        raw.append(0)
        for r, g, b, a in row:
            raw += bytes((r, g, b, a))

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(out, "wb") as f:
        f.write(png)
    print(f"wrote {out} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
