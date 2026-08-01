"""Extract only the largest frame of a save. usage: unpack2.py <save.fm> <outdir>"""
import os, sys, zstandard as zstd

save, outdir = sys.argv[1], sys.argv[2]
os.makedirs(outdir, exist_ok=True)
data = open(save, "rb").read()
off = 26
best = None  # (size, offset)
frames = []
while off < len(data) - 4:
    if data[off:off+4] != b"\x28\xb5\x2f\xfd":
        nxt = data.find(b"\x28\xb5\x2f\xfd", off)
        if nxt == -1:
            break
        off = nxt
    d = zstd.ZstdDecompressor().decompressobj()
    try:
        out = d.decompress(data[off:])
    except Exception:
        break
    consumed = len(data) - off - len(d.unused_data)
    frames.append((off, consumed, len(out)))
    if best is None or len(out) > best[0]:
        best = (len(out), off)
        open(os.path.join(outdir, "main.bin"), "wb").write(out)
    off += consumed
print(f"frames {len(frames)}, main {best[0]:,} bytes @0x{best[1]:x}")
