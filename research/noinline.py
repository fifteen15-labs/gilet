"""Find the name string-ids inside a no-inline-name person record (van Dijk)."""

import os
import re
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


# 1. string ids
def intern_id(s):
    b = s.encode()
    needle = struct.pack("<I", len(b)) + b
    at = -1
    hits = []
    while True:
        at = data.find(needle, at + 1)
        if at == -1:
            break
        sid = u32(at - 4)
        hits.append((at, sid))
    return hits


for s in ["Virgil", "van Dijk", "Declan", "Rice"]:
    hits = intern_id(s)
    print(f"{s!r}: {[(hex(a), i) for a, i in hits[:6]]}")

# 2. all person blocks via anchor, in order
blocks = []
p = 0x4000000
END = 0x6800000
while p < END:
    q = data.find(b"\x40", p)
    if q == -1 or q >= END:
        break
    p = q + 1
    t = q + 6  # triple starts 6 bytes after the 0x40
    if data[t - 3 : t] != b"\x00\x00\x00":
        continue
    a = data[t + 4 : t + 8]
    if a != data[t + 8 : t + 12] or a in (b"\x00" * 4, b"\xff" * 4):
        continue
    e = u32(t)
    if not (0 < e < 3_000_000):
        continue
    blocks.append((t, e, u32(t + 4)))

print(f"\nraw blocks: {len(blocks)}")

# LIS to keep the true ascending chain
import bisect

tails_e, tails_i = [], []
prev = [-1] * len(blocks)
for i, (t, e, x) in enumerate(blocks):
    k = bisect.bisect_left(tails_e, e)
    prev[i] = tails_i[k - 1] if k > 0 else -1
    if k == len(tails_e):
        tails_e.append(e)
        tails_i.append(i)
    elif e < tails_e[k]:
        tails_e[k] = e
        tails_i[k] = i
chain = []
i = tails_i[-1]
while i != -1:
    chain.append(blocks[i])
    i = prev[i]
chain.reverse()
print(f"LIS chain: {len(chain)}  eids {chain[0][1]}..{chain[-1][1]}")
dupes = len(chain) - len({e for _, e, _ in chain})
print(f"dupes in chain: {dupes}")

# 3. van Dijk = eid 11849
idx = next(i for i, (t, e, x) in enumerate(chain) if e == 11849)
prev_t = chain[idx - 1][0]
t, e, x = chain[idx]
print(f"\neid 11849 block at {t:#x}, uid {x}; prev block (eid {chain[idx-1][1]}) at {prev_t:#x}, span {t-prev_t}")

lo = t - 700
for row in range(lo, t + 32, 16):
    raw = data[row : row + 16]
    hx = " ".join(f"{b:02x}" for b in raw)
    asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
    print(f"{row:#010x}  {hx:<48}  {asc}")
