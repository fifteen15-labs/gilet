"""Final composition: prefix records + global-LIS identity chain + positional
binding. This is what the Rust parser implements."""

import os
import bisect
import json
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "pipeline_final.py")).read().split("# ---- 2. person prefixes ----")[0].split('exec(')[0].replace('print(f"sections', '#print(f"sections'))

big = [s for s in sections if len(s) > 10000]
forenames, surnames = big[0], big[1]

LO, HI = 0x4000000, len(data) - 30
recs = []
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

# global unanchored triple scan + LIS
blocks = []
t = LO
while t < HI:
    if data[t - 3 : t] == b"\x00\x00\x00":
        a = data[t + 4 : t + 8]
        if a == data[t + 8 : t + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e = u32(t)
            if 0 < e < 3_000_000:
                blocks.append((t, e, u32(t + 4)))
                t += 12
                continue
    t += 1
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
print(f"chain: {len(chain)}")

# bind: chain node -> record whose span contains it; record takes FIRST node.
starts = [r[0] for r in recs]
rec_eid = {}
eid2rec = {}
for t, e, x in chain:
    ri = bisect.bisect_right(starts, t) - 1
    if ri < 0:
        continue
    if ri not in rec_eid:
        rec_eid[ri] = (e, x)
        eid2rec[e] = ri

print(f"records with eid: {len(rec_eid)}  (of {len(recs)})")
eid2name = {e: recs[ri][1] for e, ri in eid2rec.items()}

gt = {10241: "Erling Braut Haaland", 6961: "Jack Peter Grealish", 11849: "Virgil van Dijk",
      16072: "Altay Bayındır", 8061: "Bukayo Ayoyinka Saka", 1: "Paolo Cesare Maldini"}
for e, want in gt.items():
    got = eid2name.get(e)
    print(f"eid {e}: {got!r} expect {want!r} {'OK' if got == want else '<<< MISMATCH'}")

squads = {int(k): v for k, v in json.load(open(os.path.join(os.path.dirname(FRAME), "squads.json"))).items()}
allm = set()
for e, (v, c, vc) in squads.items():
    allm.update(v)
res = sum(1 for m in allm if m in eid2name)
print(f"squad members {len(allm)}, resolved {res} ({res/len(allm):.2%})")

for club_eid, cn in [(369, "Manchester City"), (366, "Liverpool"), (293, "Arsenal"), (370, "Man Utd")]:
    vals, cap, vice = squads[club_eid]
    un = [v for v in vals if v not in eid2name]
    print(f"{cn}: {len(vals)} members, unresolved {un}, cap={eid2name.get(cap)}")

json.dump({str(k): v for k, v in eid2name.items()},
          open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w"))
