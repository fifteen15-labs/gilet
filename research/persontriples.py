"""Enumerate person records (name+dob acceptance, as person.rs) in the person
region, then find each record's [eid][uid][uid] triple and measure coverage."""

import os
import re
import struct

FRAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames", "f0003.bin")
data = open(FRAME, "rb").read()


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def u16(at):
    return struct.unpack_from("<H", data, at)[0]


# candidate name-length prefixes followed by an uppercase-ish letter
pat = re.compile(rb"[\x05-\x40]\x00\x00\x00[A-Z\xc3\xc4\xc5]")

people = []  # (len_at, name)
last_end = 0
for m in pat.finditer(data, 0x4000000, 0x6800000):
    at = m.start()
    if at < last_end:
        continue
    ln = data[at]
    name_b = data[at + 4 : at + 4 + ln]
    try:
        name = name_b.decode()
    except UnicodeDecodeError:
        continue
    if "\x00" in name or name.endswith(" ") or " " not in name:
        continue
    letters = sum(c.isalpha() for c in name)
    if letters < 4 or any(ord(c) < 32 for c in name):
        continue
    after = at + 4 + ln
    doy, year = u16(after), u16(after + 2)
    if not (1 <= doy <= 366 and 1920 <= year <= 2030):
        continue
    people.append((at, name))
    last_end = after

print(f"people found: {len(people)}")

# For each person, search from name-end to next person start for triples.
found = 0
multi = 0
eids = []
no_triple_examples = []
for i, (at, name) in enumerate(people):
    ln = data[at]
    start = at + 4 + ln
    end = people[i + 1][0] - 14 if i + 1 < len(people) else start + 1200
    cands = []
    p = start
    while p < end - 12:
        a = data[p + 4 : p + 8]
        if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e, x = u32(p), u32(p + 4)
            if 0 < e < 3_000_000 and x > 100_000:
                cands.append((p - at, e, x))
                p += 12
                continue
        p += 1
    if cands:
        found += 1
        if len(cands) > 1:
            multi += 1
        eids.append(cands[0][1])
    elif len(no_triple_examples) < 5:
        no_triple_examples.append((at, name))

print(f"with >=1 triple: {found}  with >1: {multi}")
mono = sum(1 for i in range(len(eids) - 1) if eids[i] < eids[i + 1])
print(f"first-triple eid monotonic: {mono}/{len(eids)-1}")
print(f"distinct eids: {len(set(eids))}")
print("no-triple examples:", [(hex(a), n) for a, n in no_triple_examples])

# stash for other scripts
import json

out = {}
for i, (at, name) in enumerate(people):
    ln = data[at]
    start = at + 4 + ln
    end = people[i + 1][0] - 14 if i + 1 < len(people) else start + 1200
    p = start
    while p < end - 12:
        a = data[p + 4 : p + 8]
        if a == data[p + 8 : p + 12] and a not in (b"\x00" * 4, b"\xff" * 4):
            e, x = u32(p), u32(p + 4)
            if 0 < e < 3_000_000 and x > 100_000:
                out[e] = name
                break
        p += 1
open(os.path.join(os.path.dirname(FRAME), "eid2name.json"), "w").write(json.dumps(out))
print("wrote eid2name.json", len(out))
