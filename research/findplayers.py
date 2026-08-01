import struct, re, os, sys

S = os.path.dirname(os.path.abspath(__file__))
d = open(os.path.join(S, "frames/f0003.bin"), "rb").read()

# Haaland record: nameid(4) 00 ff ff ff ff 00 len(4) name
# generalise: <u32 nameid> <6 bytes> <u32 len 1..64> <len printable/UTF8 bytes>
pat = re.compile(rb"[\x00-\xff]{4}\x00\xff\xff\xff\xff\x00(.{4})", re.S)

recs = []
pos = 0
while True:
    m = pat.search(d, pos)
    if not m:
        break
    pos = m.start() + 1
    ln = struct.unpack("<I", m.group(1))[0]
    if not (2 <= ln <= 64):
        continue
    nstart = m.end()
    raw = d[nstart : nstart + ln]
    try:
        name = raw.decode("utf-8")
    except UnicodeDecodeError:
        continue
    if not all(ord(c) >= 32 and ord(c) != 127 for c in name):
        continue
    if not any(c.isalpha() for c in name):
        continue
    nameid = struct.unpack("<I", d[m.start() : m.start() + 4])[0]
    recs.append((m.start(), nameid, name, nstart + ln))

print(f"candidate player-name records: {len(recs):,}")
for r in recs[:15]:
    print(f"  @{r[0]:>10}  nameid=0x{r[1]:08x}  {r[2]!r}")

names = {r[2] for r in recs}
for probe in ["Erling Braut Haaland", "Jude Bellingham", "Lamine Yamal", "Florian Wirtz"]:
    print(f"  probe {probe!r}: {'FOUND' if probe in names else 'missing'}")

import pickle
pickle.dump(recs, open(os.path.join(S, "recs.pkl"), "wb"))
print("saved recs.pkl")
