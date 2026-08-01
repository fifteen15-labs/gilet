# Football Manager 26 shortlist file format

Reverse-engineered on 1 August 2026 from two shortlists exported by FM 26 on
macOS — `Wirtz.fmf` and `WirtzNew.fmf`, 457 and 469 bytes. Cross-checked against
FM 2023 shortlists (`Football Manager 2023/cloud/shortlists/*.fmf`).

**The container is solved. The payload is encrypted, and this is where the work
deliberately stops** — see §5. Nothing here required decompiling SI code; it is
all observation of files FM wrote on the owner's own machine, the same
provenance as `SAVE_FORMAT.md`.

## 1. `.slf` did not go away — it moved inside

FM 26 has no `.slf` files on disk and no `.slf` string in its binary or its
IL2CPP metadata. That is not because the format was retired: a shortlist is now
a `.fmf` **archive** whose members include the `.slf`. FM 26 shortlists live in

```
~/Library/Application Support/Sports Interactive/Football Manager 26/shortlists/
```

alongside a plain `config.xml` that carries only `dont_scan`, not an index.

FM 26 still supports importing one. Its metadata carries `EventFmxImportShortlist`,
`EventFmxCreateShortlist`, `LoadShortlist` and `LoadIntoCurrentShortlist`.

## 2. Container — same shape as the save, different tag

Every FM auxiliary file shares the save's header shape, with a 4-character tag
naming the type:

```
offset  size  meaning
0       2     02 01            version pair, as in the save
2       4     tag              "afe." archive, "fmf." save/payload
6       3     08 00 00         format version; an older file found off-machine
                               carries 07 00 00, so treat it as a version and
                               do not assert it
9       4     u32              uncompressed length of the archive body
13      13    header remainder
26      ...   body
```

Confirmed tags: `fmf.` saves, `afe.` archives — tactics, set pieces, exported
teams and shortlists all use `afe.`. Decompressed save frames use `03 01` +
`tad.` (`SAVE_FORMAT.md` §1), so the `<u8 u8> + 4-char tag` convention is
consistent across the whole file family.

## 3. Layout — body first, manifest last

```
0x00   outer header, tag "afe."
0x1a   member blocks, back to back
 …     inner container: 02 01 "fmf." 08 00 00, then ONE zstd frame
EOF
```

The trailing inner container is only 9 bytes of header before the zstd magic —
shorter than the save's 26 — and it holds the **manifest**, not data. Decompress
it with any single-frame zstd decoder.

Manifest, all little-endian:

```
u32 len + bytes     shortlist name
u32                 member count
per member:
  u32 len + bytes   name parts, repeated until one begins with '.'
  u64               offset of the block, relative to body start (0x1a)
  u64               stored length, including the block prefix
  u64               plaintext length
  8 bytes           stamp
  8 bytes           stamp
```

`WirtzNew.fmf` declares 2 members and resolves to:

| member | offset | stored | plaintext |
|---|---|---|---|
| `image/.img` | 149 | 63 | 10 |
| `WirtzNew/.slf` | 212 | 92 | 39 |
| `_data/details.aom` | 0 | 149 | 111 |

Offsets and stored lengths tile the body exactly: 0+149 = 149, 149+63 = 212,
212+92 = 304. `_data/details.aom` is named in the manifest tail but sits outside
the declared count of 2.

## 4. Member blocks — encrypted, not compressed

Every block:

```
u32       16          constant
u32       16          constant
45 bytes  header/nonce/tag
N bytes   ciphertext, N == the manifest's plaintext length
```

`stored − plaintext` is **exactly 45** for every block in every sample, and the
ciphertext is byte-for-byte as long as the plaintext. That rules out
compression: no compressor is exactly length-preserving across a 10-byte and an
84-byte input alike, and no zstd, zlib, raw-deflate or gzip decode succeeds on
any block.

The proof it is encrypted rather than obfuscated is the `.img` member. Both
sample files carry a 10-byte plaintext `.img` in a 63-byte block — the same
default thumbnail. Their 63 stored bytes share **nothing** past the 8-byte
constant prefix, including the 45-byte header. So the nonce is random per file
and the cipher is keyed, not a fixed pad.

FM 2023's shortlists carry the identical `afe.` tag, the same `08 00 00` version
and the same `10 00 00 00 10 00 00 00` block prefix over a high-entropy body, so
this is not new in FM 26 and there is no older, plainer version of the format to
target instead.

### The decisive sample: a shortlist FM Genie Scout wrote

A shortlist produced by **FM Genie Scout**, not by FM, settles both questions —
whether a third-party tool can write an importable file, and how.

It can, and it is encrypted the same way. The file declares format version
`07 00 00` and its manifest is **zlib**, not zstd, but the block framing is
identical and its members are ciphertext:

| member | offset | stored | plaintext | difference |
|---|---|---|---|---|
| `100/.slf` | 0 | 193 | 153 | 40 |
| `shortlists/.jpg` | 193 | 21,272 | 21,232 | 40 |
| `_data/details.aom` | 21,465 | 137 | 97 | 40 |

The proof is the `.jpg`. Its manifest declares 21,232 bytes of JPEG, and a JPEG
must begin `ff d8 ff`. That sequence appears **nowhere** in the block, nor does
`JFIF` or `Exif`. Combined with a difference of exactly 40 holding across a
97-byte and a 21,232-byte payload alike, the payloads are length-preserving
ciphertext.

Two conclusions follow:

- **A third-party tool writing importable shortlists is doing it with SI's key.**
  Genie Scout's ability to export into FM is not evidence of a keyless route; it
  is evidence that the key is the route. Do not go looking for the trick Genie
  supposedly found. There isn't one.
- **Even Genie targets the older container.** Version 07 with a zlib manifest and
  40 bytes of block overhead, against FM 26's version 08 with a zstd manifest and
  53. Its timestamps are real Unix times (April 2019) where FM 26 writes a
  sentinel. This matches long-standing community reports that Genie shortlists
  stopped importing when FM moved the format on.

## 5. Why this stops here

Reading or writing a `.slf` member needs SI's cipher and key, which live in
`GameAssembly.dylib`. Extracting them is out of scope by policy, not by
difficulty:

> **Stay read-only and never circumvent anything.** […] **stop immediately if SI
> ever encrypts the save format**, because that single change flips both the UK
> and US analyses from "no technological measure exists" to "circumvention".
> — `LEGAL_NOTES.md:73`

Gilet's position rests on two facts that `LEGAL_NOTES.md` states plainly: no
technological protection measure exists (§3.8), and the parser is clean-room,
derived from observation and "never from decompiled SI code" (§8.4). Pulling a
key out of the game binary would break both at once, and it would do it for a
file that encrypts *the user's own shortlist*. Decision taken 1 August 2026:
hold the line.

**If you want to revisit this, the route is SI, not the binary.** Ask for the
format or for permission. If that ever lands, everything above is the part you
will not have to redo — the container, the manifest and the block framing are
solved, and only the cipher is missing.

## 6. Reproducing

The two sample files and the throwaway parsers are in the session scratchpad,
not the repo — the samples are game-written data and `LEGAL_NOTES.md` §1 says
never to check game data in. To regenerate:

1. In FM 26, shortlist any player, then Scouting → Shortlists → export.
2. Read the trailing manifest: find the last `02 01 "fmf."`, find the zstd magic
   `28 b5 2f fd` after it, decompress from there.
3. Walk the manifest per §3, then slice the body from offset 26 per §4.
