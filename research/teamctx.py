"""Dump context around the second co-occurrence cluster of team IDs."""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def hexdump(at, length, mark=()):
    for row in range(at, at + length, 16):
        raw = data[row : row + 16]
        hx = " ".join(f"{b:02x}" for b in raw)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
        flag = " <-- " + ",".join(str(m) for m in mark if row <= m < row + 16) if any(row <= m < row + 16 for m in mark) else ""
        print(f"{row:#010x}  {hx:<48}  {asc}{flag}")


def find_all(tid, lo=0, hi=None):
    needle = struct.pack("<I", tid)
    out = []
    at = lo - 1
    hi = hi or len(data)
    while True:
        at = data.find(needle, at + 1, hi)
        if at == -1:
            return out
        out.append(at)


# City cluster ~0x027d98ae, Arsenal cluster ~0x046b3f34
for name, ids, lo, hi in [
    ("City", [6610, 6611, 7578, 7579, 8164, 8593, 9265, 15632], 0x027D9000, 0x027DA400),
    ("Arsenal", [7027, 7108, 7714, 7715, 28560], 0x046B3800, 0x046B4600),
]:
    print(f"=== {name} cluster ===")
    marks = []
    for tid in ids:
        for at in find_all(tid, lo, hi):
            marks.append(at)
            print(f"  id {tid} at {at:#010x}")
    print()
    start = min(marks) - 96
    end = max(marks) + 96
    hexdump(start, end - start, marks)
    print()
