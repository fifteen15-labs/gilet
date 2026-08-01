"""Systematically parse club records with their pre-sig heads.

Head layout guess, reading back from the 10 ff ff sig:
  [id u32][uid u32][uid u32] 00 [nation u32][ffffffff][nation][nation][back10 u32] 00 xx 00 SIG
That is 26 + 3 + 3? — we read the 34 bytes before the sig and decode fixed slots.
"""

import os
import struct
import sys

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()

SIG = b"\x10\xff\xff"


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def parse_head(sig_at):
    # back from sig: 3 unknown bytes, back10 u32, nation u32, nation u32, ffffffff, nation u32, 00, uid u32, uid u32, id u32
    b10 = u32(sig_at - 7)
    n1 = u32(sig_at - 11)
    n2 = u32(sig_at - 15)
    ff = u32(sig_at - 19)
    n3 = u32(sig_at - 23)
    z = data[sig_at - 24]
    uid2 = u32(sig_at - 28)
    uid1 = u32(sig_at - 32)
    eid = u32(sig_at - 36)
    ok = n1 == n2 == n3 and ff == 0xFFFFFFFF and z == 0 and uid1 == uid2
    return ok, eid, uid1, n1, b10


def clubs_in(lo, hi):
    out = []
    at = lo
    while True:
        at = data.find(SIG, at + 1, hi)
        if at == -1:
            return out
        len_at = at + 3
        name_len = u32(len_at)
        if not (3 <= name_len <= 64):
            continue
        try:
            name = data[len_at + 4 : len_at + 4 + name_len].decode()
        except UnicodeDecodeError:
            continue
        if not name[:1].isupper():
            continue
        ok, eid, uid, nation, b10 = parse_head(at)
        out.append((at, name, ok, eid, uid, nation, b10))


targets = [
    (0x2E0F00, 0x2E1200),  # Arsenal
    (0x302200, 0x302400),  # Liverpool
    (0x303B00, 0x303E00),  # Man City
    (0x304600, 0x304900),  # Man Utd
    (0x4FFB00, 0x500200),  # Tomares / Badalona / Pittsburgh
]
for lo, hi in targets:
    for at, name, ok, eid, uid, nation, b10 in clubs_in(lo, hi):
        print(f"{at:#010x}  head_ok={ok}  eid={eid:>6}  uid={uid:>10}  nation={nation:>4}  back10={b10:>6}  {name}")
    print()

# Who owns entity ids 6610, 6611, 9265, 15632?  Scan the whole club region for
# heads whose eid matches.
WANT = {6610, 6611, 7578, 7579, 8164, 8593, 9265, 15632, 7027, 7108, 7714, 7715, 28560}
print("-- clubs whose head eid is in City's/Arsenal's list --")
at = -1
n = 0
while True:
    at = data.find(SIG, at + 1)
    if at == -1:
        break
    len_at = at + 3
    name_len = u32(len_at)
    if not (3 <= name_len <= 64):
        continue
    try:
        name = data[len_at + 4 : len_at + 4 + name_len].decode()
    except UnicodeDecodeError:
        continue
    if not name[:1].isupper():
        continue
    n += 1
    ok, eid, uid, nation, b10 = parse_head(at)
    if ok and eid in WANT:
        print(f"{at:#010x}  eid={eid:>6}  uid={uid:>10}  nation={nation:>4}  back10={b10:>6}  {name}")
print(f"(scanned {n} club-shaped records)")
