"""Scan every frame of a save for plain (day_of_year, year) pairs near a known
true in-game date, to find where FM 26.2.0 keeps the current date."""
import os, struct, sys
import zstandard as zstd

MAGIC = b"\x28\xb5\x2f\xfd"


def frames(path):
    data = open(path, "rb").read()
    off, out = 0, []
    while True:
        nxt = data.find(MAGIC, off)
        if nxt == -1:
            break
        d = zstd.ZstdDecompressor().decompressobj()
        try:
            raw = d.decompress(data[nxt:])
        except Exception:
            off = nxt + 4
            continue
        out.append((nxt, raw))
        off = len(data) - len(d.unused_data)
        if off <= nxt:
            off = nxt + 4
    return out


def main():
    path, year = sys.argv[1], int(sys.argv[2])
    doys = [int(x) for x in sys.argv[3].split(",")]
    names = {}
    manifest = sys.argv[4] if len(sys.argv) > 4 else None
    if manifest:
        import json
        members = json.load(open(manifest))
        # map frame index -> name by order
        names = {i: m["name"] for i, m in enumerate(members)}
    fr = frames(path)
    print(f"{len(fr)} frames")
    for i, (off, raw) in enumerate(fr):
        hits = []
        for doy in doys:
            pat = struct.pack("<HH", doy, year)
            start = 0
            while True:
                j = raw.find(pat, start)
                if j == -1:
                    break
                hits.append((doy, j))
                start = j + 1
                if len(hits) > 12:
                    break
        if hits:
            label = names.get(i, "")
            print(f"frame {i} ({label}) len {len(raw)}: {[(d, hex(h)) for d, h in hits[:12]]}")


main()
