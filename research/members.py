"""List or extract named members of an FM 26 save.

The last zstd frame of a save is a manifest naming every other frame — same
member layout as the .fmf shortlist archive (SHORTLIST_FORMAT.md section 3),
with the save name first and a trailing section of sub-archives. Offsets are
relative to byte 26 and tile the file exactly, so a member can be extracted
with one seek and one zstd frame decode. Verified against Career.fm,
Ongoing.fm and "Paul Dolden - Afan Lido.fm": every plaintext length matches
the decompressed frame byte for byte.

usage:
  uv run --with zstandard python3 members.py <save.fm>                    # list
  uv run --with zstandard python3 members.py <save.fm> <member> <out.bin> # extract
"""

import struct
import sys

import zstandard as zstd


def frames_of(data):
    """Walk the back-to-back zstd frames from byte 26; return (offset, consumed, plain_len)."""
    off, out = 26, []
    while off < len(data) - 4:
        if data[off : off + 4] != b"\x28\xb5\x2f\xfd":
            nxt = data.find(b"\x28\xb5\x2f\xfd", off)
            if nxt == -1:
                break
            off = nxt
        d = zstd.ZstdDecompressor().decompressobj()
        try:
            plain = d.decompress(data[off:])
        except zstd.ZstdError:
            break
        consumed = len(data) - off - len(d.unused_data)
        out.append((off, consumed, len(plain)))
        off += consumed
    return out


def read_members(buf, pos, n):
    """n member records: name parts until one starts with '.', then offset/stored/plain + 2 stamps."""
    out = []
    for _ in range(n):
        parts = []
        while True:
            (ln,) = struct.unpack_from("<I", buf, pos)
            s = buf[pos + 4 : pos + 4 + ln].decode()
            pos += 4 + ln
            parts.append(s)
            if s.startswith("."):
                break
        off, stored, plain = struct.unpack_from("<QQQ", buf, pos)
        pos += 24 + 16  # fields, then two 8-byte stamps (paired unix-time-like values)
        out.append(("".join(parts), off, stored, plain))
    return out, pos


def manifest_of(data):
    """Decode the trailing manifest: (save_name, members sorted by offset == frame order)."""
    frames = frames_of(data)
    d = zstd.ZstdDecompressor().decompressobj()
    buf = d.decompress(data[frames[-1][0] :])
    pos = 0
    (ln,) = struct.unpack_from("<I", buf, pos)
    save_name = buf[pos + 4 : pos + 4 + ln].decode()
    pos += 4 + ln
    (count,) = struct.unpack_from("<I", buf, pos)
    members, pos = read_members(buf, pos + 4, count)
    (nsub,) = struct.unpack_from("<I", buf, pos)
    pos += 4
    for _ in range(nsub):
        (ln,) = struct.unpack_from("<I", buf, pos)
        sub = buf[pos + 4 : pos + 4 + ln].decode()
        pos += 4 + ln
        (nchild,) = struct.unpack_from("<I", buf, pos)
        children, pos = read_members(buf, pos + 4, nchild)
        members += [(f"{sub}/{n}", o, s, p) for n, o, s, p in children]
    return save_name, sorted(members, key=lambda m: m[1])


def main():
    data = open(sys.argv[1], "rb").read()
    save_name, members = manifest_of(data)
    if len(sys.argv) == 2:
        print(f"{save_name}: {len(members)} members")
        for i, (name, off, stored, plain) in enumerate(members):
            print(f"f{i:04d}  {name:<45} @{off:>11,}  {stored:>11,} -> {plain:>11,}")
        return
    want, out_path = sys.argv[2], sys.argv[3]
    for name, off, stored, plain in members:
        if name == want or name.endswith("/" + want):
            d = zstd.ZstdDecompressor().decompressobj()
            plain_bytes = d.decompress(data[26 + off : 26 + off + stored])
            assert len(plain_bytes) == plain, (len(plain_bytes), plain)
            open(out_path, "wb").write(plain_bytes)
            print(f"{name}: {plain:,} bytes -> {out_path}")
            return
    sys.exit(f"no member named {want}")


if __name__ == "__main__":
    main()
