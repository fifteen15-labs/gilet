"""Which header offsets hold a date that MOVES between saves of one career?

The current in-game date must differ between two saves of the same career.
Anything constant across them (like frame 0 offset 0x32) is not it.
"""

import datetime
import os
import struct

import zstandard as zstd

GAMES = os.path.expanduser("~/Library/Application Support/Sports Interactive/Football Manager 26/games")

FAMILIES = {
    "Career (26.0.0)": [
        "Career.fm",
        "Career (v02) (v03).fm",
        "Career (v02) (v02).fm",
        "last save overwrite backup.fm",
        "Career (v02).fm",
    ],
    "Ongoing (26.2.0)": ["Ongoing (v03).fm", "Ongoing (v02).fm", "Ongoing.fm"],
    "Unemployed (26.2.0)": [
        "Paul Dolden - Unemployed (v03).fm",
        "Paul Dolden - Unemployed (v02).fm",
        "Paul Dolden - Unemployed.fm",
    ],
}


def frames(path, want=4):
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
        out.append(raw)
        off += len(data) - off - len(d.unused_data)
    return out


def pair_at(buf, p):
    if p + 4 > len(buf):
        return None
    doy, yr = struct.unpack_from("<HH", buf, p)
    if 1 <= doy <= 366 and 2024 <= yr <= 2060:
        return datetime.date(yr, 1, 1) + datetime.timedelta(days=doy - 1)
    return None


for family, names in FAMILIES.items():
    print(f"\n=== {family} ===")
    loaded = []
    for n in names:
        p = os.path.join(GAMES, n)
        if not os.path.exists(p):
            continue
        fr = frames(p)
        if len(fr) >= 4:
            loaded.append((n, os.path.getsize(p), fr[0], fr[3][:4096]))
    if len(loaded) < 2:
        print("  not enough saves")
        continue

    for which, idx in (("frame0", 2), ("frame3", 3)):
        span = min(len(s[idx]) for s in loaded)
        varying = []
        for off in range(0, min(span, 4096) - 4):
            dates = [pair_at(s[idx], off) for s in loaded]
            if any(d is None for d in dates):
                continue
            if len(set(dates)) > 1:
                varying.append((off, dates))
        print(f"  {which}: {len(varying)} offsets hold a date that varies")
        for off, dates in varying[:6]:
            pretty = ", ".join(f"{d.isoformat()}" for d in dates)
            print(f"    {off:#06x}: {pretty}")

    print("  file sizes (MB): " + ", ".join(f"{n.split('.fm')[0][:22]} {sz/1e6:.0f}" for n, sz, _, _ in loaded))
