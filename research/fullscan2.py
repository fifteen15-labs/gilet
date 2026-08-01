"""Person scan v2: both name layouts, string-table resolution, block eids.
Validation: resolve every member of the big-four squads."""

import os
import json
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


# --- string table --- entries [u32 id][u32 len][bytes], ids ascending within
# sections. Walk backward from a known early entry to the true start, then
# forward across the whole region, bridging small gaps between sections.
def walk_table():
    b = b"Declan"
    needle = struct.pack("<I", len(b)) + b
    at = data.find(needle)
    start = at - 4
    while True:
        found = None
        for back in range(9, 300):
            p0 = start - back
            if p0 < 0:
                break
            sid = u32(p0)
            ln = u32(p0 + 4)
            if p0 + 8 + ln == start and ln <= 250 and sid < 5_000_000:
                try:
                    data[p0 + 8 : p0 + 8 + ln].decode()
                except UnicodeDecodeError:
                    continue
                found = p0
                break
        if found is None:
            break
        start = found
    sections = []
    cur = {}
    last_id = -1
    p = start
    end = p
    gap_budget = 64
    while p + 8 < len(data):
        sid = u32(p)
        ln = u32(p + 4)
        ok = ln <= 250 and sid < 5_000_000
        s = None
        if ok:
            try:
                s = data[p + 8 : p + 8 + ln].decode()
            except UnicodeDecodeError:
                ok = False
        if ok:
            if sid < last_id and cur:
                sections.append(cur)
                cur = {}
            cur[sid] = s
            last_id = sid
            p += 8 + ln
            end = p
            gap_budget = 64
        else:
            p += 1
            gap_budget -= 1
            if gap_budget == 0:
                break
    if cur:
        sections.append(cur)
    print(f"string table: {sum(len(s) for s in sections)} entries in {len(sections)} sections, region {start:#x}..{end:#x}")
    for i, s in enumerate(sections):
        ks = sorted(s)
        print(f"  section {i}: {len(s)} ids {ks[0]}..{ks[-1]}  sample {[s[k] for k in ks[:3]]}")
    return sections


sections = walk_table()
# identify forename / surname sections by known ground truth (Haaland)
forenames = next(s for s in sections if s.get(217140) == "Erling")
surnames = next(s for s in sections if s.get(434961) == "Haaland")
print(f"forename section: {len(forenames)}  surname section: {len(surnames)}")
strings = {}  # only used for common-name fallback below

# --- person scan v2 ---
LO, HI = 0x4000000, min(0x6800000, len(data) - 30)
people = []  # (prefix_at, first_id, surname_id, common_id, inline_name, dob)
at = LO
while at < HI - 30:
    # prefix: [first u32] 00 [surname u32] 00 [common u32] 00
    if data[at + 4] != 0 or data[at + 9] != 0 or data[at + 14] != 0:
        at += 1
        continue
    first = u32(at)
    surname = u32(at + 5)
    common = u32(at + 10)
    if first == 0 or surname == 0:
        at += 1
        continue
    if first not in forenames or surname not in surnames:
        at += 1
        continue
    body = at + 15
    name = None
    ln = u32(body)
    if ln == 0:
        body += 4
    elif 2 <= ln <= 64 and u16(body + 2) == 0:
        try:
            cand = data[body + 4 : body + 4 + ln].decode()
        except UnicodeDecodeError:
            cand = None
        if cand and all(ord(c) >= 32 for c in cand):
            name = cand
            body += 4 + ln
        else:
            at += 1
            continue
    else:
        at += 1
        continue
    doy, year = u16(body), u16(body + 2)
    if not (1 <= doy <= 366 and 1920 <= year <= 2030):
        at += 1
        continue
    if name is None:
        name = forenames[first] + " " + surnames[surname]
    people.append((at, first, surname, common, name, (doy, year), body))
    at = body + 4

print(f"people v2: {len(people)}")

# --- blocks + LIS, then bind: block belongs to the person whose prefix is the
# closest preceding one.
blocks = []
p = LO
while p < HI:
    q = data.find(b"\x40", p)
    if q == -1 or q >= HI:
        break
    p = q + 1
    t = q + 6
    if data[t - 3 : t] != b"\x00\x00\x00":
        continue
    a = data[t + 4 : t + 8]
    if a != data[t + 8 : t + 12] or a in (b"\x00" * 4, b"\xff" * 4):
        continue
    e = u32(t)
    if 0 < e < 3_000_000:
        blocks.append((t, e, u32(t + 4)))

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
print(f"blocks in chain: {len(chain)}")

# bind: for each block, the latest person prefix before it
pats = [p[0] for p in people]
eid2person = {}
bind_fail = 0
for t, e, x in chain:
    k = bisect.bisect_right(pats, t) - 1
    if k < 0:
        bind_fail += 1
        continue
    eid2person[e] = k

# does each person get exactly one block?
from collections import Counter

pc = Counter(eid2person.values())
multi = sum(1 for v in pc.values() if v > 1)
print(f"eids bound: {len(eid2person)}  persons with >1 eid: {multi}  persons with 0: {len(people)-len(pc)}")

eid2name = {e: people[k][4] for e, k in eid2person.items()}
json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))

# --- resolve big-four squads ---
squads = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "squads.json"))).items()}
for club_eid, cn in [(293, "Arsenal"), (366, "Liverpool"), (369, "Manchester City"), (370, "Manchester United")]:
    vals, cap, vice = squads[club_eid]
    print(f"\n{cn} ({len(vals)}) cap={eid2name.get(cap, cap)} | vice={eid2name.get(vice, vice)}")
    for v in vals:
        print(f"   {v:>6}  {eid2name.get(v, '?? UNRESOLVED')}")

allm = set()
for e, (v, c, vc) in squads.items():
    allm.update(v)
print(f"\nall squad members {len(allm)}, resolved {sum(1 for m in allm if m in eid2name)}")
