"""Version + date candidates from the first frames of every save on disk."""

import datetime
import glob
import os
import struct

import zstandard as zstd

GAMES = os.path.expanduser("~/Library/Application Support/Sports Interactive/Football Manager 26/games")


def first_frames(path, want=4):
    data = open(path, "rb").read()
    off, out = 26, []
    while off < len(data) - 4 and len(out) < want:
        if data[off : off + 4] != b"\x28\xb5\x2f\xfd":
            nxt = data.find(b"\x28\xb5\x2f\xfd", off)
            if nxt == -1:
                break
            off = nxt
        d = zstd.ZstdDecompressor().decompressobj()
        try:
            raw = d.decompress(data[off:])
        except Exception:
            break
        out.append(raw[:400])
        off += len(data) - off - len(d.unused_data)
    return out


def dates_in(buf, lo, hi):
    out = []
    for p in range(lo, min(hi, len(buf) - 4)):
        doy, yr = struct.unpack_from("<HH", buf, p)
        if 1 <= doy <= 366 and 2024 <= yr <= 2060:
            dt = datetime.date(yr, 1, 1) + datetime.timedelta(days=doy - 1)
            out.append((hex(p), dt.isoformat()))
    return out


for path in sorted(glob.glob(os.path.join(GAMES, "*.fm"))):
    name = os.path.basename(path)
    fr = first_frames(path)
    if len(fr) < 4:
        print(f"{name}: only {len(fr)} frames")
        continue
    ver = fr[0][12:20].split(b"\x00")[0].decode("ascii", "replace")
    print(f"\n{name}  ({os.path.getsize(path)/1e6:.0f} MB)  v{ver}")
    print(f"  frame0 dates: {dates_in(fr[0], 40, 90)}")
    print(f"  frame3 dates: {dates_in(fr[3], 20, 90)}")
    print(f"  frame3 0x1a..0x36: {fr[3][0x1A:0x36].hex(' ')}")
