"""Pick each person's true triple via longest-increasing-subsequence over
candidates, then histogram the preceding bytes to derive the anchor family."""

import os
import bisect
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

# candidates per person
cands = []  # list of lists of (pos, e, x)
for i, (at, name) in enumerate(people):
    ln = data[at]
    start = at + 4 + ln
    end = people[i + 1][0] - 14 if i + 1 < len(people) else start + 1200
    cc = []
    p = start
    while p < end - 12:
        a = data[p + 4 : p + 8]
        if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e, x = u32(p), u32(p + 4)
            if 0 < e < 3_000_000 and x > 100_000:
                cc.append((p, e, x))
                p += 12
                continue
        p += 1
    cands.append(cc)

# global LIS over flattened candidates (person-ordered), one pick per person max.
# dp over candidates: value = eid; standard patience with person constraint —
# process people in order, for each keep best chain ending value.
# simple O(total * log) LIS where at most one candidate per person is chosen:
flat = []
for pi, cc in enumerate(cands):
    for (p, e, x) in cc:
        flat.append((pi, p, e, x))

# LIS on eid with strictly increasing person index AND eid: since flat is
# ordered by person then pos, do patience on eid but candidates from the same
# person must not chain: process per person, computing for each candidate the
# best chain length using state from previous people only.
import array

tails = []  # sorted eids of chain tails
tail_meta = []  # parallel: for reconstruction we store per person choices
# For simplicity: greedy two-pass — compute LIS lengths with binary search,
# snapshotting per person.
best_ends = []  # (eid, prev_index_in_chain_list) chain nodes
chain_tails = []  # sorted list of (eid) with node index
nodes = []  # (eid, person, pos, x, prev_node)


def lis_pick():
    tails_e = []  # tails_e[k] = smallest tail eid of chain of length k+1
    tails_n = []  # node index achieving it
    for pi, cc in enumerate(cands):
        updates = []
        for (p, e, x) in cc:
            k = bisect.bisect_left(tails_e, e)
            prev = tails_n[k - 1] if k > 0 else -1
            nodes.append((e, pi, p, x, prev))
            updates.append((k, e, len(nodes) - 1))
        # apply updates after processing the person (no intra-person chaining)
        for k, e, ni in updates:
            if k == len(tails_e):
                tails_e.append(e)
                tails_n.append(ni)
            elif e < tails_e[k]:
                tails_e[k] = e
                tails_n[k] = ni
    # reconstruct
    ni = tails_n[-1]
    picked = {}
    while ni != -1:
        e, pi, p, x, prev = nodes[ni]
        picked[pi] = (p, e, x)
        ni = prev
    return picked


picked = lis_pick()
print(f"people {len(people)}, picked via LIS {len(picked)}")

# anchor histogram for picked triples
pre7 = Counter()
for pi, (p, e, x) in picked.items():
    pre7[data[p - 7 : p]] += 1
print("top pre-7-byte patterns:")
for b, c in pre7.most_common(12):
    print(f"  {b.hex(' ')}  x{c}")

# eid->name map from picked
eid2name = {e: people[pi][1] for pi, (p, e, x) in picked.items()}
json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))
print(f"eid2name entries: {len(eid2name)}")

# how many people were NOT picked (their record contributes no eid)
missing = [pi for pi in range(len(people)) if pi not in picked]
print(f"unpicked people: {len(missing)}")
for pi in missing[:10]:
    print("   ", hex(people[pi][0]), people[pi][1], "cands:", len(cands[pi]))

# re-resolve the big four squads
squads = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "squads.json"))).items()}
for club_eid, cn in [(293, "Arsenal"), (369, "Man City")]:
    vals, cap, vice = squads[club_eid]
    unresolved = [v for v in vals if v not in eid2name]
    print(f"\n{cn}: {len(vals)} players, unresolved {len(unresolved)} {unresolved}")
