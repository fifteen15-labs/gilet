"""Follow the eight u32s after each club's short name — presumed team IDs.

Step 1: verify the count-byte + u32-array pattern across clubs.
Step 2: for a few known clubs, scan the frame for places where several of one
        club's team IDs co-occur closely — that is where team records live.
"""

import os
import struct
import sys

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()
print(f"frame size {len(data):,}")

SIG = b"\x10\xff\xff"


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def find_clubs(names):
    """Locate club records by full name, return {name: (len_at, club_id, short_end)}."""
    out = {}
    for name in names:
        needle = struct.pack("<I", len(name)) + name.encode()
        at = -1
        while True:
            at = data.find(needle, at + 1)
            if at == -1:
                break
            if data[at - 3 : at] != SIG:
                continue
            club_id = u32(at - 10)
            short_len = u32(at + 4 + len(name))
            if not (2 <= short_len <= 32):
                continue
            short = data[at + 8 + len(name) : at + 8 + len(name) + short_len]
            short_end = at + 8 + len(name) + short_len
            out[name] = (at, club_id, short_end, short.decode("utf-8", "replace"))
            break
    return out


def team_array(short_end):
    """Parse `01, six bytes, 02, count, count*u32` after the short name."""
    at = short_end
    if data[at] != 0x01:
        return None, data[at : at + 24].hex(" ")
    if data[at + 7] != 0x02:
        return None, data[at : at + 24].hex(" ")
    count = data[at + 8]
    ids = [u32(at + 9 + 4 * i) for i in range(count)]
    return ids, None


CLUBS = [
    "Manchester City",
    "Arsenal",
    "Liverpool",
    "Manchester United",
    "Chelsea",
    "Real Madrid",
    "FC Barcelona",
    "FC Bayern München",
]

clubs = find_clubs(CLUBS)
teams = {}
for name, (len_at, club_id, short_end, short) in clubs.items():
    ids, raw = team_array(short_end)
    print(f"\n{name} (id {club_id}, short {short!r}, len_at {len_at:#x})")
    if ids is None:
        print(f"  pattern mismatch, bytes: {raw}")
    else:
        print(f"  {len(ids)} team ids: {ids}")
        teams[name] = ids

# Step 2: where do a club's team IDs co-occur? Slide a window over the frame
# counting distinct team-id hits for the club; report dense spots.
print("\n--- co-occurrence scan ---")
for name in ["Manchester City", "Arsenal"]:
    if name not in teams:
        continue
    ids = teams[name]
    hits = []  # (offset, id)
    for tid in ids:
        needle = struct.pack("<I", tid)
        at = -1
        while True:
            at = data.find(needle, at + 1)
            if at == -1:
                break
            hits.append((at, tid))
    hits.sort()
    print(f"\n{name}: {len(hits)} raw u32 hits for {len(ids)} ids")
    # cluster: hits within 2000 bytes of each other with >= 3 distinct ids
    i = 0
    clusters = []
    while i < len(hits):
        j = i
        while j + 1 < len(hits) and hits[j + 1][0] - hits[j][0] < 2000:
            j += 1
        distinct = {tid for _, tid in hits[i : j + 1]}
        if len(distinct) >= 3:
            clusters.append((hits[i][0], hits[j][0], sorted(distinct), j - i + 1))
        i = j + 1
    for lo, hi, distinct, n in clusters[:40]:
        print(f"  {lo:#010x}..{hi:#010x}  span {hi-lo:>6}  {n:>3} hits  ids {distinct}")
    if len(clusters) > 40:
        print(f"  ... {len(clusters) - 40} more clusters")
