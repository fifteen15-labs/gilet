"""Validate the contract anchor at scale: for each person with an eid, search
the 220 bytes before their prefix for [eid][u32][0000][wage] 01 ?? 00 ffffffff,
and an 8xff run followed by a date pair (expiry)."""

import os
import json
import struct
import datetime
from collections import Counter

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


# people + eids, quick rebuild via pipeline pieces (prefix scan + triples LIS)
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "pipeline_v2.py")).read().split("# bind:")[0].replace('print(f"prefix records', '#p(f"prefix records').replace('print(f"chain', '#p(f"chain'))

import bisect

starts = [r[0] for r in recs]
rec_eid = {}
for t, e, x in chain:
    ri = bisect.bisect_right(starts, t) - 1
    if ri >= 0 and ri not in rec_eid:
        rec_eid[ri] = e

print(f"people {len(recs)}, with eid {len(rec_eid)}")


def contract_for(prefix_at, eid):
    lo = max(0, prefix_at - 220)
    needle = struct.pack("<I", eid)
    p = data.rfind(needle, lo, prefix_at)
    if p == -1:
        return None
    # structure: [eid][u32][00 00 00 00][wage u32] 01 ?? 00 ff ff ff ff
    if data[p + 8 : p + 12] != b"\x00\x00\x00\x00":
        return None, None, p
    wage = u32(p + 12)
    if data[p + 16] != 0x01 or data[p + 18] != 0x00 or data[p + 19 : p + 23] != b"\xff\xff\xff\xff":
        return None, None, p
    # expiry: 8xff then date pair, somewhere between lo and p
    expiry = None
    q = data.rfind(b"\xff" * 8, lo, p)
    if q != -1:
        d, y = u16(q + 8), u16(q + 10)
        if 1 <= d <= 366 and 2020 <= y <= 2060:
            expiry = (d, y)
    return wage, expiry, p


found = 0
wage_ok = 0
expiry_ok = 0
wages = []
years = Counter()
samples = {}
for ri, e in rec_eid.items():
    at = recs[ri][0]
    r = contract_for(at, e)
    if r is None:
        continue
    wage, expiry, p = r
    found += 1
    if wage is not None:
        wage_ok += 1
        wages.append(wage)
        if expiry:
            expiry_ok += 1
            years[expiry[1]] += 1
        samples[recs[ri][1]] = (wage, expiry)

print(f"eid found in window: {found}")
print(f"wage structure valid: {wage_ok}")
print(f"with expiry: {expiry_ok}")
wages.sort()
if wages:
    n = len(wages)
    print(f"wage percentiles: p10 {wages[n//10]:,} p50 {wages[n//2]:,} p90 {wages[9*n//10]:,} max {wages[-1]:,}")
print("expiry years:", dict(sorted(years.items())))

for name in ["Erling Braut Haaland", "Bukayo Ayoyinka Saka", "Mohamed Salah Ghaly",
             "Virgil van Dijk", "Kylian Mbappé Lottin", "Jude Victor William Bellingham"]:
    if name in samples:
        w, ex = samples[name]
        exs = ""
        if ex:
            exs = (datetime.date(ex[1], 1, 1) + datetime.timedelta(days=ex[0] - 1)).isoformat()
        print(f"  {name}: £{w:,}/wk  until {exs}")
    else:
        print(f"  {name}: no contract found")
