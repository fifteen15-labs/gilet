"""Tighten the person-triple rule via the 40-XX-04 anchor; rescan squad table
by head shape rather than terminator chaining."""

import os
import json
import re
import struct
from collections import Counter

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


pat = re.compile(rb"[\x05-\x40]\x00\x00\x00[A-Z\xc3\xc4\xc5]")
people = []
last_end = 0
for m in pat.finditer(data, 0x4000000, 0x6800000):
    at = m.start()
    if at < last_end:
        continue
    ln = data[at]
    try:
        name = data[at + 4 : at + 4 + ln].decode()
    except UnicodeDecodeError:
        continue
    if "\x00" in name or name.endswith(" ") or " " not in name:
        continue
    if sum(c.isalpha() for c in name) < 4 or any(ord(c) < 32 for c in name):
        continue
    after = at + 4 + ln
    if not (1 <= u16(after) <= 366 and 1920 <= u16(after + 2) <= 2030):
        continue
    people.append((at, name))
    last_end = after

print(f"people: {len(people)}")


def find_triple(start, end, anchored_only):
    p = start
    while p < end - 12:
        a = data[p + 4 : p + 8]
        if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e, x = u32(p), u32(p + 4)
            if 0 < e < 3_000_000 and x > 100_000:
                anchored = data[p - 6] == 0x40 and data[p - 4] == 0x04 and data[p - 3 : p] == b"\x00\x00\x00"
                if anchored or not anchored_only:
                    return p, e, x, anchored
        p += 1
    return None


stats = Counter()
eids = []
eid2name = {}
misses = []
for i, (at, name) in enumerate(people):
    ln = data[at]
    start = at + 4 + ln
    end = people[i + 1][0] - 14 if i + 1 < len(people) else start + 1200
    r = find_triple(start, end, anchored_only=True)
    if r is None:
        r = find_triple(start, end, anchored_only=False)
        if r is None:
            stats["none"] += 1
            continue
        stats["fallback"] += 1
        if len(misses) < 8:
            misses.append((hex(at), name))
    else:
        stats["anchored"] += 1
    p, e, x, _ = r
    eids.append(e)
    eid2name[e] = name

print(dict(stats))
mono = sum(1 for i in range(len(eids) - 1) if eids[i] < eids[i + 1])
print(f"monotonic: {mono}/{len(eids)-1}  distinct {len(set(eids))}")
print("fallback examples:", misses)

json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))

# ---- squad table by head scan ----
print("\n-- squad table heads --")
heads = []
# head: [eid u32][00 x10][idx u32][uid u32][uid u32]; scan region around the table
LO, HI = 0x1E00000, 0x2300000
p = LO
zero10 = b"\x00" * 10
while True:
    p = data.find(zero10, p + 1, HI)
    if p == -1:
        break
    at = p - 4
    eid = u32(at)
    if not (1 <= eid <= 200_000):
        continue
    uid = u32(at + 18)
    if uid == 0 or uid == 0xFFFFFFFF or u32(at + 22) != uid:
        continue
    heads.append((at, eid, uid))
    p += 10

# keep the longest strictly-increasing run (the table itself)
print(f"raw heads: {len(heads)}")
best_run = []
run = [heads[0]]
for h in heads[1:]:
    if h[1] > run[-1][1]:
        run.append(h)
    else:
        if len(run) > len(best_run):
            best_run = run
        run = [h]
if len(run) > len(best_run):
    best_run = run
print(f"longest increasing run: {len(best_run)}  eid {best_run[0][1]}..{best_run[-1][1]}  at {best_run[0][0]:#x}..{best_run[-1][0]:#x}")


def parse_squad(at, nxt):
    for p in range(at + 26, nxt - 6):
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
for i, (at, eid, uid) in enumerate(best_run):
    nxt = best_run[i + 1][0] if i + 1 < len(best_run) else at + 3000
    vals, cap, vice = parse_squad(at, nxt)
    squads[eid] = (vals, cap, vice)

with_squad = [e for e, (v, _, _) in squads.items() if v]
print(f"clubs in table: {len(squads)}  with squads: {len(with_squad)}")
sizes = Counter(len(squads[e][0]) for e in with_squad)
print(f"sizes top: {sizes.most_common(10)}")

json.dump({str(k): v for k, v in squads.items()},
          open(os.path.join(os.path.dirname(FRAME), "squads.json"), "w"))

# cross-check with names
for club, ceid in [("Arsenal", 293), ("Liverpool", 366), ("Man City", 369), ("Man Utd", 370)]:
    vals, cap, vice = squads.get(ceid, ([], None, None))
    names = [eid2name.get(v, f"?{v}") for v in vals]
    capn = eid2name.get(cap, cap)
    vicen = eid2name.get(vice, vice)
    print(f"\n{club} ({len(vals)}): cap={capn} vice={vicen}")
    print("  " + "; ".join(names[:36]))
