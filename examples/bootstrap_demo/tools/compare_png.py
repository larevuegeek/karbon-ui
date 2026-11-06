"""Side-by-side and pixel difference of two PNG screenshots (standard library only).

    python -I examples/bootstrap_demo/tools/compare_png.py REF.png NATIVE.png OUT_side_by_side.png [--diff OUT_diff.png]

Prints the share of pixels whose channels all differ by at most 16 and 48 (out of 255),
the mean absolute difference, and the same figures by horizontal band of the page.
Images of different sizes are compared on their common area.
"""
import argparse
import struct
import zlib
from pathlib import Path


def read_png(path):
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, chunks, idat = 8, {}, b""
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            chunks["ihdr"] = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    width, height, depth, color, _, _, interlace = chunks["ihdr"]
    assert depth == 8 and interlace == 0, "8-bit non-interlaced PNG expected"
    channels = {2: 3, 6: 4, 0: 1, 4: 2}[color]
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev = [], bytearray(stride)
    i = 0
    for _ in range(height):
        f = raw[i]
        line = bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        if f == 1:
            for x in range(channels, stride):
                line[x] = (line[x] + line[x - channels]) & 255
        elif f == 2:
            for x in range(stride):
                line[x] = (line[x] + prev[x]) & 255
        elif f == 3:
            for x in range(stride):
                left = line[x - channels] if x >= channels else 0
                line[x] = (line[x] + ((left + prev[x]) >> 1)) & 255
        elif f == 4:
            for x in range(stride):
                a = line[x - channels] if x >= channels else 0
                b = prev[x]
                c = prev[x - channels] if x >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pred) & 255
        rows.append(bytes(line))
        prev = line
    # Conversion en RGB.
    out = []
    for line in rows:
        if channels == 3:
            out.append(line)
        elif channels == 4:
            rgb = bytearray(width * 3)
            rgb[0::3], rgb[1::3], rgb[2::3] = line[0::4], line[1::4], line[2::4]
            out.append(bytes(rgb))
        elif channels == 1:
            out.append(bytes(b for v in line for b in (v, v, v)))
        else:
            out.append(bytes(b for v in line[0::2] for b in (v, v, v)))
    return width, height, out


def write_png(path, width, height, rows):
    raw = b"".join(b"\x00" + r for r in rows)
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b"")
    Path(path).write_bytes(png)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("reference")
    parser.add_argument("native")
    parser.add_argument("out")
    parser.add_argument("--diff")
    parser.add_argument("--bands", type=int, default=8)
    args = parser.parse_args()
    rw, rh, ref = read_png(args.reference)
    nw, nh, nat = read_png(args.native)
    w, h = min(rw, nw), min(rh, nh)
    gap = 16
    side = []
    white = b"\xff" * (gap * 3)
    for y in range(max(rh, nh)):
        a = ref[y] if y < rh else b"\xff" * (rw * 3)
        b = nat[y] if y < nh else b"\xff" * (nw * 3)
        side.append(a + white + b)
    write_png(args.out, rw + gap + nw, max(rh, nh), side)
    total = close16 = close48 = 0
    absdiff = 0
    band = max(1, h // args.bands)
    bands = []
    diff_rows = []
    for y in range(h):
        a, b = ref[y], nat[y]
        row16 = row48 = 0
        drow = bytearray(w * 3)
        for x in range(w):
            i = x * 3
            d = max(abs(a[i] - b[i]), abs(a[i + 1] - b[i + 1]), abs(a[i + 2] - b[i + 2]))
            absdiff += d
            if d <= 16:
                row16 += 1
            if d <= 48:
                row48 += 1
            v = min(255, d * 2)
            drow[i] = 255
            drow[i + 1] = drow[i + 2] = 255 - v
        diff_rows.append(bytes(drow))
        close16 += row16
        close48 += row48
        total += w
        if y % band == 0:
            bands.append([0, 0, 0])
        bands[-1][0] += w
        bands[-1][1] += row16
        bands[-1][2] += row48
    if args.diff:
        write_png(args.diff, w, h, diff_rows)
    print(f"compared {w}x{h} px: within 16/255 {100 * close16 / total:.1f}%, within 48/255 {100 * close48 / total:.1f}%, mean |diff| {absdiff / total:.2f}")
    for i, (n, c16, c48) in enumerate(bands):
        print(f"  band {i}: rows {i * band}-{min(h, (i + 1) * band) - 1}: within 16 {100 * c16 / n:.1f}%, within 48 {100 * c48 / n:.1f}%")


if __name__ == "__main__":
    main()
