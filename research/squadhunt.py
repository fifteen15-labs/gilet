"""Find person uids, then hunt for co-occurrence clusters = squad lists."""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def person_ids(name_sub):
    """Find a person by name substring; return (full_at, eid, uid) or None."""
    needle = name_sub.encode()
    at = -1
    while True:
        at = data.find(needle, at + 1)
        if at == -1:
            return None
        # walk back up to 48 bytes to find a length prefix that lands here
        for back in range(4, 52):
            ln = u32(at - back)
            if not (2 <= ln <= 64):
                continue
            start = at - back + 4
            end = start + ln
            if not (start <= at and at + len(needle) <= end):
                continue
            # full name must be valid utf8 text
            try:
                full = data[start:end].decode()
            except UnicodeDecodeError:
                continue
            # find [e][x][x] within 500 bytes after name end
            for p in range(end, end + 500):
                a = data[p + 4 : p + 8]
                if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
                    e, x = u32(p), u32(p + 4)
                    if 0 < e < 200_000 and 20_000_000 < x < 3_000_000_000:
                        return full, start - 4, e, x
        # keep looking at next occurrence


CITY = ["Erling Braut Haaland", "Jack Peter Grealish", "Kyle Andrew Walker",
        "Philip Walter Foden", "Rodrigo Hernández", "Gato Alves Dias",
        "Ederson Santana", "Veiga de Carvalho e Silva", "Savio Moreira", "Oscar Bobb"]
OTHER = ["Bukayo Ayoyinka Saka", "Declan Rice", "Mohamed Salah", "van Dijk",
         "Alexander Isak"]

people = {}
for nm in CITY + OTHER:
    r = person_ids(nm)
    if r is None:
        print(f"{nm}: NOT FOUND")
        continue
    full, off, e, x = r
    tag = "CITY " if nm in CITY else "other"
    people[full] = (tag, off, e, x)
    print(f"{tag} {full}: rec@{off:#x}  eid {e}  uid {x}")

# Now: every occurrence of each uid and eid across the file.
print("\n-- occurrences of each uid outside the person record --")
occs = []
for full, (tag, off, e, x) in people.items():
    for val, kind in [(x, "uid")]:
        needle = struct.pack("<I", val)
        at = -1
        spots = []
        while True:
            at = data.find(needle, at + 1)
            if at == -1:
                break
            if abs(at - off) < 2000:
                continue  # inside own record
            spots.append(at)
        occs.append((full, tag, kind, val, spots))
        locs = " ".join(f"{s:#x}" for s in spots[:12])
        print(f"{tag} {full} {kind} {val}: {len(spots)} hits  {locs}")

# cluster: any window of 200KB containing uids of >=4 CITY players and no others?
print("\n-- co-occurrence of CITY uids --")
allspots = []
for full, tag, kind, val, spots in occs:
    for s in spots:
        allspots.append((s, tag, full))
allspots.sort()
i = 0
while i < len(allspots):
    j = i
    while j + 1 < len(allspots) and allspots[j + 1][0] - allspots[j][0] < 100_000:
        j += 1
    group = allspots[i : j + 1]
    city = {f for _, t, f in group if t == "CITY "}
    other = {f for _, t, f in group if t == "other"}
    if len(city) >= 3:
        print(f"{group[0][0]:#x}..{group[-1][0]:#x}: {len(city)} city ({sorted(city)}), {len(other)} other ({sorted(other)})")
    i = j + 1
