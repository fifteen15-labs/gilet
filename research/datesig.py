"""Histogram the (doy, year) pairs that follow the `01 00 6c 07 01` signature
(null date 1/1900, then 0x01) in each save's main db frame. If the current
date dominates on the ground-truth saves, this is the 26.2.0 date reader."""
import struct, sys
from collections import Counter
import zstandard as zstd

MAGIC = b"\x28\xb5\x2f\xfd"
SIG = bytes.fromhex("01006c0701")


def main_frame(path):
    data = open(path, "rb").read()
    off, best = 26, None
    while off < len(data) - 4:
        nxt = data.find(MAGIC, off)
        if nxt == -1:
            break
        d = zstd.ZstdDecompressor().decompressobj()
        try:
            raw = d.decompress(data[nxt:])
        except Exception:
            break
        if best is None or len(raw) > len(best):
            best = raw
        off = nxt + (len(data) - nxt - len(d.unused_data))
    return best


for path in sys.argv[1:]:
    raw = main_frame(path)
    counts = Counter()
    start = 0
    while True:
        j = raw.find(SIG, start)
        if j == -1:
            break
        start = j + 1
        if j + 9 > len(raw):
            break
        doy, year = struct.unpack_from("<HH", raw, j + 5)
        if not (1 <= doy <= 366 and 2000 <= year <= 2100):
            continue
        # The real stamp writes the same pair 25 bytes earlier in the record.
        h = j + 5
        if h >= 25 and raw[h - 25 : h - 21] == raw[h : h + 4]:
            counts[(doy, year)] += 1
    total = sum(counts.values())
    top = counts.most_common(6)
    name = path.rsplit("/", 1)[-1]
    share = (top[0][1] / total * 100) if top else 0
    print(f"{name}: {total} valid-date signature hits; top {top}  ({share:.0f}%)")
