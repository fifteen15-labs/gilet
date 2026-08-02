"""Histogram the byte before `FF FF` on records that otherwise match the club
head shape (nation id three times with FFFFFFFF between, then club id), to
learn every value of the record-signature byte scan_clubs must accept."""
import struct, sys
from collections import Counter
import zstandard as zstd

MAGIC = b"\x28\xb5\x2f\xfd"


def main_frame(path):
    data = open(path, "rb").read()
    off, best = 26, None
    while off < len(data) - 4:
        nxt = data.find(MAGIC, off)
        if nxt == -1:
            break
        d = zstd.ZstdDecompressor().decompressobj()
        try:
            raw = d.decompress(data[nxt:])
        except Exception:
            break
        if best is None or len(raw) > len(best):
            best = raw
        off = nxt + (len(data) - nxt - len(d.unused_data))
    return best


def plausible_name(raw, at):
    if at + 4 > len(raw):
        return False
    (ln,) = struct.unpack_from("<I", raw, at)
    if not 3 <= ln <= 64:
        return False
    chunk = raw[at + 4 : at + 4 + ln]
    if len(chunk) != ln:
        return False
    try:
        text = chunk.decode()
    except UnicodeDecodeError:
        return False
    return text[:1].isupper() and not any(c.isdigit() or ord(c) < 32 for c in text)


path = sys.argv[1]
raw = main_frame(path)
counts = Counter()
examples = {}
start = 0
while True:
    j = raw.find(b"\xff\xff\xff\xff", start)  # the FF run inside the head
    if j == -1:
        break
    start = j + 1
    # len_at - 22 == j  =>  len_at = j + 22
    len_at = j + 22
    if len_at + 4 > len(raw) or len_at < 39:
        continue
    (n3,) = struct.unpack_from("<I", raw, len_at - 26)
    (n2,) = struct.unpack_from("<I", raw, len_at - 18)
    (n1,) = struct.unpack_from("<I", raw, len_at - 14)
    if not (n1 == n2 == n3) or n1 == 0 or n1 > 100000:
        continue
    if raw[len_at - 27] != 0:
        continue
    if not plausible_name(raw, len_at):
        continue
    sig = raw[len_at - 3 : len_at]
    if sig[1:] != b"\xff\xff":
        continue
    counts[sig[0]] += 1
    if sig[0] not in examples:
        (ln,) = struct.unpack_from("<I", raw, len_at)
        examples[sig[0]] = raw[len_at + 4 : len_at + 4 + ln].decode()
print(f"{path.rsplit('/',1)[-1]}: {sum(counts.values())} head-shaped records")
for b, n in sorted(counts.items()):
    print(f"  sig 0x{b:02x}: {n:6}   e.g. {examples[b]!r}")
