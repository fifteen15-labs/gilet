"""Dump the bytes before a named club's record in the main db frame, to see
why the [eid][uid][uid] head fails to parse for some clubs."""
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
        if best is None or len(raw) > len(best):
            best = raw
        off = nxt + (len(data) - nxt - len(d.unused_data))
    return best


path, name = sys.argv[1], sys.argv[2].encode()
raw = main_frame(path)
needle = struct.pack("<I", len(name)) + name
start = 0
while True:
    j = raw.find(needle, start)
    if j == -1:
        break
    start = j + 1
    back = raw[j - 48 : j]
    print(f"name-len @0x{j:x}")
    print(f"  -48..0: {back.hex(' ')}")
    # decode the expected head fields
    for label, off in [("eid", 39), ("uid", 35), ("uid2", 31), ("zero", 27),
                       ("nat3", 26), ("ff", 22), ("nat2", 18), ("nat1", 14),
                       ("club_id", 10), ("sig", 3)]:
        if off >= 4:
            (v,) = struct.unpack_from("<I", raw, j - off)
            print(f"  {label:8} @-{off}: {v} (0x{v:08x})")
        else:
            print(f"  {label:8} @-{off}: {raw[j-off:j].hex(' ')}")
