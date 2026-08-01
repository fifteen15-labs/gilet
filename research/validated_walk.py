"""Walk the squad table validating each record against the club table's
(eid, uid) pairs. Also dump a staff record to find its triple anchor."""

import os
import json
import struct
from collections import Counter

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


# 1. club map from the club table
SIG = b"\x10\xff\xff"
clubs = {}
at = -1
while True:
    at = data.find(SIG, at + 1)
    if at == -1:
        break
    ln = u32(at + 3)
    if not (3 <= ln <= 64):
        continue
    try:
        name = data[at + 7 : at + 7 + ln].decode()
    except UnicodeDecodeError:
        continue
    if not name[:1].isupper():
        continue
    n1, n2, n3 = u32(at - 11), u32(at - 15), u32(at - 23)
    if not (n1 == n2 == n3 and u32(at - 19) == 0xFFFFFFFF and data[at - 24] == 0):
        continue
    uid1, uid2, eid = u32(at - 28), u32(at - 32), u32(at - 36)
    if uid1 != uid2:
        continue
    clubs[eid] = (uid1, name)
print(f"clubs with valid heads: {len(clubs)}")

# 2. validated squad-table walk
START = 0x1ED2ECE
zero10 = b"\x00" * 10


def is_head(q):
    if data[q + 4 : q + 14] != zero10:
        return False
    eid = u32(q)
    uid = u32(q + 18)
    if uid == 0 or uid == 0xFFFFFFFF or u32(q + 22) != uid:
        return False
    c = clubs.get(eid)
    return c is not None and c[0] == uid


heads = []
q = START
misses = 0
while q < len(data) - 30:
    if is_head(q):
        heads.append(q)
        q += 26
        misses = 0
        continue
    q += 1
    misses += 1
    if misses > 20000:  # table ended
        break
print(f"squad-table heads: {len(heads)}  span {heads[0]:#x}..{heads[-1]:#x}")
eids = [u32(h) for h in heads]
mono = sum(1 for i in range(len(eids) - 1) if eids[i] < eids[i + 1])
print(f"monotonic: {mono}/{len(eids)-1}  first {eids[0]} last {eids[-1]}")


def parse_squad(at, nxt):
    for p in range(at + 26, min(nxt, at + 4000) - 6):
        if data[p : p + 4] != b"\xff\xff\xff\xff":
            continue
        n = u16(p + 4)
        if not (1 <= n <= 80):
            continue
        end = p + 6 + 4 * n
        if end > nxt:
            continue
        vals = [u32(p + 6 + 4 * i) for i in range(n)]
        if any(v == 0 or v > 3_000_000 for v in vals):
            continue
        asc = sum(1 for i in range(min(6, n - 1)) if vals[i] < vals[i + 1])
        if n >= 3 and asc < min(6, n - 1) - 1:
            continue
        cap = u32(end) if end + 4 <= nxt else None
        vice = u32(end + 4) if end + 8 <= nxt else None
        return vals, cap, vice
    return [], None, None


squads = {}
for i, h in enumerate(heads):
    nxt = heads[i + 1] if i + 1 < len(heads) else h + 3000
    vals, cap, vice = parse_squad(h, nxt)
    squads[eids[i]] = (vals, cap, vice)

with_squad = [e for e, (v, _, _) in squads.items() if v]
print(f"clubs with squads: {len(with_squad)}")
sizes = Counter(len(squads[e][0]) for e in with_squad)
print(f"sizes: {sizes.most_common(12)}")

eid2name = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "eid2name.json"))).items()}
for club_eid in (293, 366, 369, 370):
    vals, cap, vice = squads.get(club_eid, ([], None, None))
    cn = clubs[club_eid][1]
    print(f"\n{cn} ({len(vals)}) cap={eid2name.get(cap, cap)} vice={eid2name.get(vice, vice)}")
    print("  " + "; ".join(eid2name.get(v, f"?{v}") for v in vals))

json.dump({str(k): v for k, v in squads.items()},
          open(os.path.join(os.path.dirname(FRAME), "squads.json"), "w"))

# 3. staff record dump: Maldini
print("\n-- Maldini record --")
at = 0x412623C
for row in range(at - 16, at + 320, 16):
    raw = data[row : row + 16]
    hx = " ".join(f"{b:02x}" for b in raw)
    asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
    print(f"{row:#010x}  {hx:<48}  {asc}")
