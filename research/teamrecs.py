"""Dump context around every [tid][uid][uid] record for one team."""

import os
import struct
import sys

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()

TID = int(sys.argv[1]) if len(sys.argv) > 1 else 6610
BEFORE = int(sys.argv[2]) if len(sys.argv) > 2 else 256
AFTER = int(sys.argv[3]) if len(sys.argv) > 3 else 512


def hexdump(at, length, marks=()):
    for row in range(at, at + length, 16):
        raw = data[row : row + 16]
        hx = " ".join(f"{b:02x}" for b in raw)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
        m = " <==" if any(row <= x < row + 16 for x in marks) else ""
        print(f"{row:#010x}  {hx:<48}  {asc}{m}")


needle = struct.pack("<I", TID)
at = -1
while True:
    at = data.find(needle, at + 1)
    if at == -1:
        break
    a = data[at + 4 : at + 8]
    if a != data[at + 8 : at + 12] or a in (b"\x00" * 4, b"\xff" * 4):
        continue
    uid = struct.unpack("<I", a)[0]
    print(f"\n===== tid {TID} at {at:#010x}, uid {uid} =====")
    hexdump(at - BEFORE, BEFORE + AFTER, [at])
