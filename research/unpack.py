import os, sys, zstandard as zstd

SAVE = os.path.expanduser(
    "~/Library/Application Support/Sports Interactive/Football Manager 26/games/Career.fm"
)
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames")
os.makedirs(OUT, exist_ok=True)

data = open(SAVE, "rb").read()
print(f"file size      {len(data):,}")
print(f"outer header   {data[:26].hex(' ')}")

off = 26
idx = 0
total = 0
sizes = []
while off < len(data) - 4:
    if data[off : off + 4] != b"\x28\xb5\x2f\xfd":
        # not at a frame start; scan forward
        nxt = data.find(b"\x28\xb5\x2f\xfd", off)
        if nxt == -1:
            print(f"no more frames after offset {off:,}")
            break
        print(f"  gap: {nxt-off} non-frame bytes at 0x{off:x}: {data[off:min(nxt,off+32)].hex(' ')}")
        off = nxt
    d = zstd.ZstdDecompressor().decompressobj()
    try:
        out = d.decompress(data[off:])
    except Exception as e:
        print(f"  frame {idx} @0x{off:x} FAILED: {e}")
        break
    consumed = len(data) - off - len(d.unused_data)
    open(os.path.join(OUT, f"f{idx:04d}.bin"), "wb").write(out)
    sizes.append((idx, off, consumed, len(out)))
    total += len(out)
    if idx < 10:
        print(f"  frame {idx:4d} @0x{off:08x} comp={consumed:>10,} raw={len(out):>10,}  head={out[:12].hex(' ')}")
    off += consumed
    idx += 1

print(f"\nframes: {idx}   decompressed total: {total:,}  ({total/len(data):.1f}x)")
big = sorted(sizes, key=lambda s: -s[3])[:10]
print("largest frames (idx, offset, comp, raw):")
for b in big:
    print(f"   {b[0]:4d}  0x{b[1]:08x}  {b[2]:>10,}  {b[3]:>10,}")
