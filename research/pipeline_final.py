"""The exact pipeline the Rust parser will implement. Measures final quality."""

import os
import bisect
import json
import struct
from collections import Counter

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


# ---- 1. string table (from fullscan2) ----
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
    gap = 64
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
            gap = 64
        else:
            p += 1
            gap -= 1
            if gap == 0:
                break
    if cur:
        sections.append(cur)
    return sections


sections = walk_table()
big = [s for s in sections if len(s) > 10000]
forenames, surnames = big[0], big[1]
commons = big[2] if len(big) > 2 else {}
print(f"sections {len(sections)}: forenames {len(forenames)} surnames {len(surnames)} commons {len(commons)}")

# ---- 2. person prefixes ----
LO, HI = 0x4000000, len(data) - 30
recs = []  # (prefix_at, name, dob)
at = LO
while at < HI:
    if data[at + 4] != 0 or data[at + 9] != 0 or data[at + 14] != 0:
        at += 1
        continue
    first = u32(at)
    surname = u32(at + 5)
    if first == 0 or surname == 0 or first not in forenames or surname not in surnames:
        at += 1
        continue
    body = at + 15
    ln = u32(body)
    name = None
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
    recs.append((at, name, (doy, year)))
    at = body + 4

print(f"prefix records: {len(recs)}")

# ---- 3. identity triples per record + LIS (<=1 per record) ----
starts = [r[0] for r in recs]
nodes = []  # (rec_idx, eid, uid)
for i, (at, name, dob) in enumerate(recs):
    end = starts[i + 1] if i + 1 < len(recs) else min(at + 1500, HI)
    p = at + 15
    while p < end - 12:
        if data[p - 3 : p] == b"\x00\x00\x00":
            a = data[p + 4 : p + 8]
            if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
                e = u32(p)
                if 0 < e < 3_000_000:
                    nodes.append((i, e, u32(p + 4)))
                    p += 12
                    continue
        p += 1

tails_e, tails_n = [], []
prev = [-1] * len(nodes)
cur_rec = -1
pending = []
node_list = []
for ni, (ri, e, x) in enumerate(nodes):
    if ri != cur_rec:
        for k, e2, ni2 in pending:
            if k == len(tails_e):
                tails_e.append(e2)
                tails_n.append(ni2)
            elif e2 < tails_e[k]:
                tails_e[k] = e2
                tails_n[k] = ni2
        pending = []
        cur_rec = ri
    k = bisect.bisect_left(tails_e, e)
    prev[ni] = tails_n[k - 1] if k > 0 else -1
    pending.append((k, e, ni))
for k, e2, ni2 in pending:
    if k == len(tails_e):
        tails_e.append(e2)
        tails_n.append(ni2)
    elif e2 < tails_e[k]:
        tails_e[k] = e2
        tails_n[k] = ni2

picked = {}
ni = tails_n[-1]
while ni != -1:
    ri, e, x = nodes[ni]
    picked[ri] = (e, x)
    ni = prev[ni]
print(f"records with eid: {len(picked)}")

eid2name = {e: recs[ri][1] for ri, (e, x) in picked.items()}

gt = {10241: "Erling Braut Haaland", 6961: "Jack Peter Grealish", 11849: "Virgil van Dijk",
      16072: None, 8061: "Bukayo Ayoyinka Saka", 1: "Paolo Cesare Maldini"}
for e, want in gt.items():
    got = eid2name.get(e)
    print(f"eid {e}: {got!r}" + (f"  expect {want!r} {'OK' if got == want else '<<< MISMATCH'}" if want else ""))

squads = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "squads.json"))).items()}
allm = set()
for e, (v, c, vc) in squads.items():
    allm.update(v)
res = sum(1 for m in allm if m in eid2name)
print(f"squad members {len(allm)}, resolved {res} ({res/len(allm):.1%})")

for club_eid, cn in [(369, "Manchester City"), (366, "Liverpool")]:
    vals, cap, vice = squads[club_eid]
    un = [v for v in vals if v not in eid2name]
    print(f"{cn}: {len(vals)} members, unresolved {un}, cap={eid2name.get(cap)}, vice={eid2name.get(vice)}")

json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))
