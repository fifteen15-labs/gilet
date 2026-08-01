"""Dump a club's full record body and extract the typed sub-object pointers."""

import os
import re
import struct
import sys

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def hexdump(at, length):
    for row in range(at, at + length, 16):
        raw = data[row : row + 16]
        hx = " ".join(f"{b:02x}" for b in raw)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
        print(f"{row:#010x}  {hx:<48}  {asc}")


def find_club(name):
    needle = struct.pack("<I", len(name)) + name.encode()
    at = -1
    while True:
        at = data.find(needle, at + 1)
        if at == -1:
            return None
        if data[at - 3 : at] == b"\x10\xff\xff":
            return at
    return None


name = sys.argv[1] if len(sys.argv) > 1 else "Manchester City"
len_at = find_club(name)
print(f"{name} len_at {len_at:#x}")

# Head: the 16 bytes before the sig should be [id][uid][uid] 00 [nation] ...
print("\n-- 48 bytes before sig --")
hexdump(len_at - 48 - 3, 48)

# Body: from end of short name to next sig
short_at = len_at + 4 + len(name)
short_len = u32(short_at)
body_at = short_at + 4 + short_len
nxt = data.find(b"\x10\xff\xff", body_at)
print(f"\nbody {body_at:#x} .. next sig {nxt:#x}  (len {nxt - body_at})")

# Sub-object pointer rows: 01 TT [u32 x][u32 x] with x repeated
print("\n-- typed pointers (01 TT uid uid) in body --")
for at in range(body_at, nxt - 10):
    if data[at] == 0x01 and 0x02 <= data[at + 1] <= 0x0F:
        a = data[at + 2 : at + 6]
        if a == data[at + 6 : at + 10] and a not in (b"\x00" * 4, b"\xff" * 4):
            print(f"  {at:#010x}  type {data[at+1]:>2}  uid {u32(at+2):>12}  ({u32(at+2):#x})")

print("\n-- first 640 bytes of body --")
hexdump(body_at, 640)
