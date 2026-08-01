"""Dump the file region where an unresolved eid must live (between the records
of its resolved neighbours)."""

import os
import json
import re
import struct
import sys

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


# rebuild rows (offset, name, first anchored eid) quickly
pat = re.compile(rb"[\x05-\x40]\x00\x00\x00[A-Z\xc3\xc4\xc5]")
rows = []
last_end = 0
for m in pat.finditer(data, 0x4000000, 0x6800000):
    at = m.start()
    if at < last_end:
        continue
    ln = data[at]
    try:
        name = data[at + 4 : at + 4 + ln].decode()
    except UnicodeDecodeError:
        continue
    if "\x00" in name or name.endswith(" ") or " " not in name:
        continue
    if sum(c.isalpha() for c in name) < 4 or any(ord(c) < 32 for c in name):
        continue
    after = at + 4 + ln
    if not (1 <= u16(after) <= 366 and 1920 <= u16(after + 2) <= 2030):
        continue
    rows.append((at, name))
    last_end = after


def first_anchored(idx):
    at, _ = rows[idx]
    ln = data[at]
    start = at + 4 + ln
    end = rows[idx + 1][0] - 14 if idx + 1 < len(rows) else start + 1200
    p = start + 7
    out = []
    while p < end - 12:
        if data[p - 6] == 0x40 and data[p - 3 : p] == b"\x00\x00\x00":
            a = data[p + 4 : p + 8]
            if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
                e = u32(p)
                if 0 < e < 3_000_000:
                    out.append(e)
                    p += 12
                    continue
        p += 1
    return out


want = int(sys.argv[1]) if len(sys.argv) > 1 else 12656

# find neighbour records whose anchored eid brackets `want`
lo_i = hi_i = None
for i in range(len(rows)):
    es = first_anchored(i)
    if not es:
        continue
    e = min(es)
    if e < want:
        lo_i = (i, e)
    if e > want and hi_i is None:
        hi_i = (i, e)
        break

print(f"want {want}: below = {rows[lo_i[0]][1]} eid {lo_i[1]} at {rows[lo_i[0]][0]:#x}")
print(f"          above = {rows[hi_i[0]][1]} eid {hi_i[1]} at {rows[hi_i[0]][0]:#x}")
gap = hi_i[0] - lo_i[0]
print(f"records between: {gap - 1}")

# raw dump between the two records
lo_at = rows[lo_i[0]][0]
hi_at = rows[hi_i[0]][0]
if hi_at - lo_at < 4000:
    for row in range(lo_at, hi_at + 48, 16):
        raw = data[row : row + 16]
        hx = " ".join(f"{b:02x}" for b in raw)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
        print(f"{row:#010x}  {hx:<48}  {asc}")
else:
    # search for [want][x][x] directly in the gap
    needle = struct.pack("<I", want)
    p = lo_at
    while True:
        p = data.find(needle, p + 1, hi_at)
        if p == -1:
            print("eid u32 not found in gap")
            break
        if data[p + 4 : p + 8] == data[p + 8 : p + 12]:
            print(f"triple at {p:#x}")
            for row in range(p - 160, p + 48, 16):
                raw = data[row : row + 16]
                hx = " ".join(f"{b:02x}" for b in raw)
                asc = "".join(chr(b) if 32 <= b < 127 else "." for b in raw)
                print(f"{row:#010x}  {hx:<48}  {asc}")
            break
