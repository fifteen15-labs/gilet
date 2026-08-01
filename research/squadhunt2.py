"""(a) club eid/uid near player records; (b) clusters of person eids."""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


PLAYERS = {
    "Haaland": (0x48E8AC6, 10241, 29179241, 369, 678),
    "Grealish": (0x46024F9, 6961, 28067800, 369, 678),
    "Walker": (0x457528A, 6357, 28009441, None, None),
    "Saka": (0x47022A7, 8061, 28122642, 293, 601),
    "Salah": (0x501DBDE, 18461, 98028755, 366, 675),
}

print("-- (a) club eid / club uid near each player record --")
for nm, (at, eid, uid, club_eid, club_uid) in PLAYERS.items():
    lo, hi = at - 1000, at + 3000
    for label, val in [("club_eid", club_eid), ("club_uid", club_uid)]:
        if val is None:
            continue
        needle = struct.pack("<I", val)
        spots = []
        p = lo - 1
        while True:
            p = data.find(needle, p + 1, hi)
            if p == -1:
                break
            spots.append(p - at)
        print(f"{nm:>9} {label} {val:>4}: offsets rel to name-len {spots}")

print("\n-- (b) clusters of City person eids --")
CITY_EIDS = [10241, 6961, 6357, 15856, 14078, 14042]
OTHER_EIDS = [8061, 18461, 40740]
hits = []
for e in CITY_EIDS + OTHER_EIDS:
    needle = struct.pack("<I", e)
    p = -1
    while True:
        p = data.find(needle, p + 1)
        if p == -1:
            break
        hits.append((p, e))
hits.sort()
print(f"total raw hits {len(hits)}")
i = 0
while i < len(hits):
    j = i
    while j + 1 < len(hits) and hits[j + 1][0] - hits[j][0] < 400:
        j += 1
    group = hits[i : j + 1]
    city = {e for _, e in group if e in CITY_EIDS}
    other = {e for _, e in group if e in OTHER_EIDS}
    if len(city) >= 3:
        print(f"{group[0][0]:#x}..{group[-1][0]:#x}  city {sorted(city)}  other {sorted(other)}")
    i = j + 1
