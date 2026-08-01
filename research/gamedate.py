"""Hunt the in-game date in header frames, with ground truth for both saves."""
import datetime, os, struct, sys, zstandard as zstd

GAMES = os.path.expanduser("~/Library/Application Support/Sports Interactive/Football Manager 26/games")
TRUTH = {"Career.fm": datetime.date(2025,10,26), "Ongoing.fm": datetime.date(2035,5,28)}

def frames(path, limit=6):
    data = open(path,"rb").read()
    off, out = 26, []
    while off < len(data)-4 and len(out) < limit:
        if data[off:off+4] != b"\x28\xb5\x2f\xfd":
            nxt = data.find(b"\x28\xb5\x2f\xfd", off)
            if nxt == -1: break
            off = nxt
        d = zstd.ZstdDecompressor().decompressobj()
        try: raw = d.decompress(data[off:])
        except Exception: break
        out.append((off, raw))
        off += len(data) - off - len(d.unused_data)
    return data[:26], out

for name, truth in TRUTH.items():
    path = os.path.join(GAMES, name)
    if not os.path.exists(path): continue
    head26, fr = frames(path)
    doy = truth.timetuple().tm_yday
    print(f"\n=== {name}  truth {truth} (doy {doy}, year {truth.year}) ===")
    print(f"outer 26 bytes: {head26.hex(' ')}")
    for i,(off,raw) in enumerate(fr[:4]):
        print(f"  frame {i} @0x{off:x} len {len(raw)}: {raw[:96].hex(' ')}")
    # search each frame for encodings
    pair = struct.pack("<HH", doy, truth.year)
    epochs = {
        "days1900": (truth - datetime.date(1900,1,1)).days,
        "days1970": (truth - datetime.date(1970,1,1)).days,
        "days2000": (truth - datetime.date(2000,1,1)).days,
        "days1601": (truth - datetime.date(1601,1,1)).days,
    }
    for i,(off,raw) in enumerate(fr):
        if pair in raw:
            print(f"  frame {i}: (doy,year) pair at {[hex(j) for j in range(len(raw)) if raw[j:j+4]==pair][:5]}")
        for label, v in epochs.items():
            n32 = struct.pack("<I", v)
            hits = [j for j in range(len(raw)-4) if raw[j:j+4]==n32]
            if hits:
                print(f"  frame {i}: {label}={v} at {[hex(h) for h in hits[:5]]}")
