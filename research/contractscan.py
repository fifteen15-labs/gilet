"""Scan for the contract-block shape: [team_id u32][uid u32][same uid u32].

If counts per team are squad-sized, this is the club->player link.
"""

import os
import struct
from collections import defaultdict

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()

TEAMS = {
    "Man City": [6610, 6611, 7578, 7579, 8164, 8593, 9265, 15632],
    "Arsenal": [7027, 7108, 7714, 7715, 28560],
    "Liverpool": [2995, 7539, 7749, 16946, 16947, 16948, 36579, 36580],
    "Chelsea": [23097, 23098, 23105, 23106, 23107, 23108, 23109, 23111, 23112],
}

hits = defaultdict(list)  # tid -> [(offset, uid)]
for club, ids in TEAMS.items():
    for tid in ids:
        needle = struct.pack("<I", tid)
        at = -1
        while True:
            at = data.find(needle, at + 1)
            if at == -1:
                break
            a = data[at + 4 : at + 8]
            b = data[at + 8 : at + 12]
            if a == b and a != b"\x00\x00\x00\x00" and a != b"\xff\xff\xff\xff":
                uid = struct.unpack("<I", a)[0]
                hits[tid].append((at, uid))

for club, ids in TEAMS.items():
    print(f"\n{club}")
    for tid in ids:
        rows = hits[tid]
        uids = sorted({u for _, u in rows})
        print(f"  team {tid:>6}: {len(rows):>4} hits, {len(uids):>4} distinct uids", end="")
        if rows:
            lo = min(o for o, _ in rows)
            hi = max(o for o, _ in rows)
            print(f"  offsets {lo:#x}..{hi:#x}  uid range {min(uids)}..{max(uids)}")
            if len(uids) <= 40:
                print(f"      uids: {uids}")
        else:
            print()
