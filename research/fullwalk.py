"""Full squad-table walk + person-eid extraction at scale + cross-checks."""

import os
import struct
from collections import Counter

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


TERM = b"\x00\x00\x00\x00\xff\xff\xff\xff\xff\x00\x00\x00\x00\x64\xff"


def parse_rec(at):
    eid = u32(at)
    if data[at + 4 : at + 14] != b"\x00" * 10:
        return None
    uid = u32(at + 18)
    if u32(at + 22) != uid:
        return None
    term = data.find(TERM, at + 26, at + 6000)
    if term == -1:
        return None
    nxt = term + 16
    squad, cap, vice = [], None, None
    for p in range(at + 26, term):
        if data[p : p + 4] != b"\xff\xff\xff\xff":
            continue
        n = u16(p + 4)
        if not (1 <= n <= 80):
            continue
        end = p + 6 + 4 * n
        if end > term:
            continue
        vals = [u32(p + 6 + 4 * i) for i in range(n)]
        if any(v == 0 or v > 3_000_000 for v in vals):
            continue
        asc = sum(1 for i in range(min(6, n - 1)) if vals[i] < vals[i + 1])
        if n >= 3 and asc < min(6, n - 1) - 1:
            continue
        squad = vals
        cap = u32(end) if end + 4 <= term else None
        vice = u32(end + 4) if end + 8 <= term else None
        break
    return eid, uid, squad, cap, vice, nxt


# walk the whole table
START = 0x1ED2ECE
at = START
recs = {}
fails = 0
last_eid = 0
while True:
    r = parse_rec(at)
    if r is None:
        break
    eid, uid, squad, cap, vice, nxt = r
    recs[eid] = (uid, squad, cap, vice, at)
    if eid < last_eid:
        print(f"NON-MONOTONIC eid {eid} after {last_eid} at {at:#x}")
    last_eid = eid
    at = nxt

print(f"table records: {len(recs)}, end at {at:#x}, last eid {last_eid}")
with_squad = [e for e, (_, s, _, _, _) in recs.items() if s]
print(f"records with a squad list: {len(with_squad)}")
sizes = Counter(len(recs[e][1]) for e in with_squad)
print(f"squad sizes (top): {sizes.most_common(8)}")

# person-eid extraction for every person: anchor 02 40 ?? 04 00 00 00
print("\n-- person triples via anchor --")
import re

triples = []
pat = re.compile(rb"\x02\x40.\x04\x00\x00\x00", re.DOTALL)
for m in pat.finditer(data):
    p = m.end()
    a = data[p + 4 : p + 8]
    if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
        e, x = u32(p), u32(p + 4)
        if 0 < e < 3_000_000:
            triples.append((p, e, x))
print(f"anchored triples: {len(triples)}")
eids = [e for _, e, _ in triples]
print(f"distinct eids: {len(set(eids))}")
mono = sum(1 for i in range(len(triples) - 1) if triples[i][1] < triples[i + 1][1])
print(f"monotonic adjacent pairs: {mono}/{len(triples)-1}")

# cross-checks
eid2off = {e: p for p, e, x in triples}
print("\n-- cross-checks --")
for club, ceid, want_in, want_out in [
    ("Arsenal", 293, [8061], []),
    ("Liverpool", 366, [18461, 40740], []),
    ("Man City", 369, [10241, 6961, 14042, 14078, 15856], [6357]),
]:
    uid, squad, cap, vice, _ = recs[ceid]
    ok_in = all(w in squad for w in want_in)
    ok_out = all(w not in squad for w in want_out)
    print(f"{club}: squad {len(squad)} cap {cap} vice {vice} contains-wanted {ok_in} excludes {ok_out}")

# how many squad members resolve to an anchored person eid?
all_members = set()
for e in with_squad:
    all_members.update(recs[e][1])
resolved = sum(1 for m in all_members if m in eid2off)
print(f"\nsquad members total distinct {len(all_members)}, resolved to person triples {resolved}")
