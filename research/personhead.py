"""Check whether person records carry an [eid][uid][uid] head like clubs do."""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def hexdump(at, length, mark=None):
    for row in range(at, at + length, 16):
        raw = data[row : row + 16]
        hx = " ".join(f"{b:02x}" for b in raw)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
        m = " <== name" if mark is not None and row <= mark < row + 16 else ""
        print(f"{row:#010x}  {hx:<48}  {asc}{m}")


for name in [
    "Erling Braut Haaland",
    "Bukayo Ayoyinka Saka",
    "Kevin De Bruyne",
    "Philip Charles Harris",
]:
    needle = struct.pack("<I", len(name)) + name.encode()
    at = -1
    print(f"===== {name} =====")
    while True:
        at = data.find(needle, at + 1)
        if at == -1:
            break
        # print 80 bytes before the length prefix, 16 after
        print(f"-- occurrence at {at:#010x} --")
        hexdump(at - 80, 96, at)
        # decode the head-candidate: look for x,x repeated u32 pair in the 40 bytes before
        for back in range(4, 60):
            p = at - back
            a = data[p : p + 4]
            if a == data[p + 4 : p + 8] and a not in (b"\x00" * 4, b"\xff" * 4):
                print(f"   repeated u32 at -{back}: {u32(p)}")
        print()
