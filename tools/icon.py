"""Draws icon.png, the registry icon: a brass compass whose needle's north end is a leaf, swinging north-east, in a
vanilla tooltip's frame (the frame LookAt's icon uses, so the mods look like one family). 32 x 32 pixel art scaled
8 times to 256 x 256. Needs Pillow.

    python tools/icon.py [icon.png]
"""
import math
import sys

from PIL import Image

SIZE, SCALE = 32, 8
BACKGROUND = (0x10, 0x00, 0x10, 0xF0)
FRAME_TOP = (0x50, 0x00, 0xFF, 0xFF)
FRAME_BOTTOM = (0x28, 0x00, 0x7F, 0xFF)


def lerp(a, b, t):
    return tuple(round(ca + (cb - ca) * t) for ca, cb in zip(a, b))


class Canvas:
    """A 32 x 32 tooltip; drawing stays inside its frame."""

    def __init__(self):
        self.img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        self.px = self.img.load()
        for y in range(SIZE):
            for x in range(SIZE):
                if x in (0, SIZE - 1) and y in (0, SIZE - 1):
                    continue
                edge = x in (0, SIZE - 1) or y in (0, SIZE - 1)
                frame = x in (1, SIZE - 2) or y in (1, SIZE - 2)
                self.px[x, y] = lerp(FRAME_TOP, FRAME_BOTTOM, (y - 1) / (SIZE - 3)) if frame and not edge else BACKGROUND

    def set(self, x, y, c):
        if not (2 <= x < SIZE - 2 and 2 <= y < SIZE - 2) or c is None:
            return
        if len(c) == 4 and c[3] < 255:
            under, a = self.px[x, y], c[3] / 255
            c = tuple(round(u + (v - u) * a) for u, v in zip(under[:3], c[:3])) + (under[3],)
        self.px[x, y] = c if len(c) == 4 else c + (255,)

    def sprite(self, x0, y0, art, palette):
        for dy, row in enumerate(art):
            for dx, ch in enumerate(row):
                if ch not in ". ":
                    self.set(x0 + dx, y0 + dy, palette[ch])

    def save(self, path):
        self.img.resize((SIZE * SCALE, SIZE * SCALE), Image.NEAREST).save(path, optimize=True)


PALETTE = {
    # the case: outline, brass, its light and shadow
    "K": (0x22, 0x16, 0x10), "B": (0xC8, 0x8C, 0x34), "b": (0xF0, 0xC0, 0x5C), "d": (0x80, 0x52, 0x1C),
    # the face, its rim's shade and its ticks
    "F": (0xF0, 0xE8, 0xD0), "f": (0xD0, 0xC4, 0xA4), "t": (0x7C, 0x6C, 0x58),
    # the needle's leaf: light, body, shadow, vein; its other end: light, shadow; the pin
    "g": (0x8C, 0xD8, 0x5C), "G": (0x4C, 0xA8, 0x34), "L": (0x2A, 0x6C, 0x22), "v": (0xC8, 0xF0, 0x98),
    "W": (0xE0, 0xE0, 0xEA), "w": (0x90, 0x90, 0xA0), "P": (0x3A, 0x2A, 0x20),
}
CX, CY = 16.0, 17.5

# The ring the compass hangs by, 6 x 4, over the top of the case.
RING = [
    ".KKKK.",
    "KbBBdK",
    "Kb..dK",
    "KB..dK",
]


def draw():
    c = Canvas()
    c.sprite(13, 3, RING, PALETTE)
    # the case and its face, lit from the top left
    for y in range(SIZE):
        for x in range(SIZE):
            dx, dy = x + 0.5 - CX, y + 0.5 - CY
            r = math.hypot(dx, dy)
            if r > 11.6:
                continue
            if r > 10.6:
                colour = "K"
            elif r > 8.6:
                colour = "b" if dx + dy < -6 else "d" if dx + dy > 6 else "B"
            elif r > 7.7:
                colour = "f"
            else:
                colour = "F"
            c.set(x, y, PALETTE[colour])
    # the ticks of the four points and the four between
    for a in range(0, 360, 45):
        rad = math.radians(a)
        length = 2 if a % 90 == 0 else 1
        for k in range(length):
            r = 7.2 - k
            c.set(math.floor(CX + r * math.sin(rad)), math.floor(CY - r * math.cos(rad)), PALETTE["t"])
    # the needle: along its axis (north-east > 0) and across it
    for y in range(SIZE):
        for x in range(SIZE):
            px, py = x + 0.5 - CX, y + 0.5 - CY
            along = (px - py) / math.sqrt(2)
            across = (px + py) / math.sqrt(2)
            if along >= 0:
                # a leaf: widest a third of the way out, to a point at the tip
                length = 10.0
                if along > length:
                    continue
                t = along / length
                half = 2.7 * math.sin(math.pi * t ** 0.7) + 0.4
                if abs(across) > half:
                    continue
                if abs(across) < 0.5 and along > 1.0:
                    colour = "v"
                elif abs(across) > half - 0.9:
                    colour = "L"
                else:
                    colour = "g" if across < 0 else "G"
            else:
                length = 7.5
                if -along > length:
                    continue
                half = 1.6 * (1 + along / length) + 0.3
                if abs(across) > half:
                    continue
                colour = "W" if across < 0 else "w"
            c.set(x, y, PALETTE[colour])
    for x, y in [(15, 17), (16, 17), (15, 16), (16, 16)]:
        c.set(x, y, PALETTE["P"])
    return c


if __name__ == "__main__":
    draw().save(sys.argv[1] if len(sys.argv) > 1 else "icon.png")
