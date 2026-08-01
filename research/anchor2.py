"""Final person-eid rule test: anchor byte -6 == 0x40, -3..-1 == 0, any uid."""

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

stats = Counter()
rows = []  # (person_idx, eid, uid) first anchored candidate
for i, (at, name) in enumerate(people):
    ln = data[at]
    start = at + 4 + ln
    end = people[i + 1][0] - 14 if i + 1 < len(people) else start + 1200
    got = []
    p = start + 7
    while p < end - 12:
        if data[p - 6] == 0x40 and data[p - 3 : p] == b"\x00\x00\x00":
            a = data[p + 4 : p + 8]
            if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
                e = u32(p)
                if 0 < e < 3_000_000:
                    got.append((p, e, u32(p + 4)))
                    p += 12
                    continue
        p += 1
    stats[min(len(got), 3)] += 1
    if got:
        rows.append((i, got[0][1], got[0][2], [g[1] for g in got]))

print("candidate-count histogram:", dict(stats))
eids = [r[1] for r in rows]
mono = sum(1 for i in range(len(eids) - 1) if eids[i] < eids[i + 1])
print(f"first-anchored monotonic: {mono}/{len(eids)-1}  distinct {len(set(eids))}")

# where are the breaks?
breaks = [(rows[i], rows[i + 1]) for i in range(len(rows) - 1) if eids[i] >= eids[i + 1]]
for a, b in breaks[:10]:
    print("break:", people[a[0]][1], a[1], a[3], "->", people[b[0]][1], b[1], b[3])

eid2name = {r[1]: people[r[0]][1] for r in rows}
json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))

squads = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "squads.json"))).items()}
for club_eid, cn in [(293, "Arsenal"), (366, "Liverpool"), (369, "Man City"), (370, "Man Utd")]:
    vals, cap, vice = squads[club_eid]
    unresolved = [v for v in vals if v not in eid2name]
    print(f"{cn}: {len(vals)} players, unresolved {len(unresolved)} {unresolved}")

# global: how many squad members across all clubs resolve?
allm = set()
for e, (v, c, vc) in squads.items():
    allm.update(v)
print(f"all squad members {len(allm)}, resolved {sum(1 for m in allm if m in eid2name)}")
