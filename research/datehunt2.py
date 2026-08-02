"""Locate the repeated current-date stamps in the main db frame and show their
record context, to find a structural anchor for reading the date."""
import struct, sys
import zstandard as zstd

MAGIC = b"\x28\xb5\x2f\xfd"


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
        consumed = len(data) - nxt - len(d.unused_data)
        if best is None or len(raw) > len(best):
            best = raw
        off = nxt + consumed
    return best


path, year, doy = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
raw = main_frame(path)
print(f"main frame {len(raw)} bytes")
pat = struct.pack("<HH", doy, year)
hits, start = [], 0
while True:
    j = raw.find(pat, start)
    if j == -1:
        break
    hits.append(j)
    start = j + 1
print(f"{len(hits)} hits for (doy {doy}, {year})")
for i, h in enumerate(hits[:40]):
    delta = h - hits[i - 1] if i else 0
    print(f"  0x{h:08x}  (+{delta})  {raw[h-16:h+8].hex(' ')}")
