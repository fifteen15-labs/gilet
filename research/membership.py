"""Does a player's record contain [club_eid][person_uid][person_uid]?"""

import os
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


CASES = [
    ("Erling Braut Haaland", 369),  # Man City
    ("Bukayo Ayoyinka Saka", 293),  # Arsenal
    ("Josko Gvardiol", 369),
    ("Jack Peter Grealish", 369),
    ("Kyle Andrew Walker", None),
    ("Alisson Ramsés Becker", 366),  # Liverpool
    ("Virgil van Dijk", 366),
]

for name, want in CASES:
    needle = struct.pack("<I", len(name)) + name.encode()
    at = data.find(needle)
    print(f"== {name} (expect club eid {want}) ==")
    if at == -1:
        print("   name not found")
        continue
    # scan forward up to 900 bytes from the name for [u32 e][u32 x][u32 x]
    end = at + 4 + len(name) + 900
    found = []
    for p in range(at, end):
        a = data[p + 4 : p + 8]
        if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e = u32(p)
            x = u32(p + 4)
            if 0 < e < 100_000 and x > 500:
                found.append((p - at, e, x))
    for off, e, x in found:
        tag = "  <== MATCH" if want is not None and e == want else ""
        print(f"   +{off:>4}: eid {e:>6}  uid {x}{tag}")
    print()
