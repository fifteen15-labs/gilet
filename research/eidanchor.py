"""(1) Context around the [eid][uid][uid] triple in person records.
(2) Bounds of the squad table."""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


PLAYERS = {
    "Haaland": (0x48E8AC6, 10241),
    "Saka": (0x47022A7, 8061),
    "Walker": (0x457528A, 6357),
    "Salah": (0x501DBDE, 18461),
    "vanDijk": (0x5FDCC45, 40740),
    "Grealish": (0x46024F9, 6961),
}

print("-- 32 bytes before each person's own [eid][uid][uid] triple --")
for nm, (at, eid) in PLAYERS.items():
    needle = struct.pack("<I", eid)
    p = at
    while True:
        p = data.find(needle, p + 1, at + 1500)
        if p == -1:
            print(f"{nm}: triple not found")
            break
        if data[p + 4 : p + 8] == data[p + 8 : p + 12] and data[p + 4 : p + 8] not in (b"\x00" * 4, b"\xff" * 4):
            pre = data[p - 32 : p]
            post = data[p + 12 : p + 28]
            print(f"{nm:>9} rel +{p-at:>4}: pre {pre.hex(' ')}")
            print(f"{'':>9}            post {post.hex(' ')}")
            break

# (2) table bounds: walk records both directions from City's record.
print("\n-- squad table walk --")


def parse_rec(at):
    """at points at club eid. Returns (eid, uid, count, list, cap, vice, next_at) or None."""
    eid = u32(at)
    if data[at + 4 : at + 14] != b"\x00" * 10:
        return None
    uid = u32(at + 18)
    if u32(at + 22) != uid:
        return None
    # find terminator: 00 00 00 00 ff ff ff ff ff 00 00 00 00 64 ff
    term = data.find(b"\x00\x00\x00\x00\xff\xff\xff\xff\xff\x00\x00\x00\x00\x64\xff", at + 26, at + 4000)
    if term == -1:
        return None
    nxt = term + 16  # skip term + 1 mode byte
    # find list: ff ff ff ff [u16 n] [n ascending u32s]
    best = None
    for p in range(at + 26, term):
        if data[p : p + 4] != b"\xff\xff\xff\xff":
            continue
        n = struct.unpack_from("<H", data, p + 4)[0]
        if not (1 <= n <= 80):
            continue
        vals = [u32(p + 6 + 4 * i) for i in range(n)]
        if any(v == 0 or v > 3_000_000 for v in vals):
            continue
        asc = sum(1 for i in range(min(6, n - 1)) if vals[i] < vals[i + 1])
        if n >= 3 and asc < min(6, n - 1) - 1:
            continue
        end = p + 6 + 4 * n
        if end > term:
            continue
        cap = u32(end) if end + 4 <= term else None
        vice = u32(end + 4) if end + 8 <= term else None
        best = (n, vals, cap, vice)
        break
    if best:
        n, vals, cap, vice = best
        return eid, uid, n, vals, cap, vice, nxt
    return eid, uid, 0, [], None, None, nxt


# walk forward from a known record start
CITY_REC = 0x1EE03FC
at = CITY_REC
last_eid = None
count_recs = 0
while count_recs < 6:
    r = parse_rec(at)
    if r is None:
        print(f"parse fail at {at:#x}")
        break
    eid, uid, n, vals, cap, vice, nxt = r
    print(f"rec@{at:#x} eid {eid} uid {uid} squad {n} cap {cap} vice {vice} first5 {vals[:5]}")
    at = nxt
    count_recs += 1

# find the table's first record: search backwards for eid=1 record head shape
print("\n-- searching for table start (eid small, head shape) --")
head1 = struct.pack("<I", 1) + b"\x00" * 10
p = data.rfind(head1, 0x1C00000, 0x1EE1000)
while p != -1:
    if data[p + 18 : p + 22] == data[p + 22 : p + 26]:
        print(f"candidate eid=1 head at {p:#x}, uid {u32(p+18)}")
        break
    p = data.rfind(head1, 0x1C00000, p)
