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
212+92 = 304. `_data/details.aom` sits outside the declared count of 2 because
the manifest has **two** sections: the declared members, then a second `u32`
count followed by FM's own internal members. Both samples declare 2 user
members and 1 internal one, and the manifest ends with `00 00 00 00`.

The "plaintext length" is the length before compression, not the length of the
bytes stored in the block — see §4.

## 4. Member blocks — a zstd frame, encrypted

Every block:

```
u32       16          constant — reads as the nonce length
u32       16          constant — reads as the tag length
16 bytes  nonce       random per file
16 bytes  tag         authentication tag
N bytes   ciphertext, N == stored − 40
```

**The overhead is a constant 40 bytes, not 45.** The earlier figure was wrong
and it took the reasoning with it. The plaintext length in the manifest is the
length *before* compression, so the ciphertext is not the same size as the
plaintext, and the difference is not noise — it is exactly zstd framing:

| file | member | stored | plaintext | payload (stored−40) | payload − plaintext |
|---|---|---|---|---|---|
| `Wirtz` | `_data/details.aom` | 146 | 108 | 106 | −2 |
| `Wirtz` | `image/.img` | 63 | 10 | 23 | **+13** |
| `Wirtz` | `Wirtz/.slf` | 84 | 31 | 44 | **+13** |
| `WirtzNew` | `_data/details.aom` | 149 | 111 | 109 | −2 |
| `WirtzNew` | `image/.img` | 63 | 10 | 23 | **+13** |
| `WirtzNew` | `WirtzNew/.slf` | 92 | 39 | 52 | **+13** |

A zstd frame that stores its content raw costs exactly 13 bytes — 4 magic, 1
frame header descriptor, 1 window descriptor, 3 block header, 4 content
checksum. Three payloads of different sizes (10, 31 and 39 bytes) each land on
+13, which is what an incompressible input does; the two `details.aom` members
are large enough to actually compress and come in 2 bytes under. So the block
is `encrypt(zstd(plaintext))` with a fixed 40 bytes of crypto overhead.

This also explains the older container in §4's Genie table, where the
difference is exactly 40 on all three members including a 21,232-byte JPEG:
version 07 has the same 40-byte crypto overhead and **no** zstd layer. The
compression was added in version 08.

**It is still encrypted.** The zstd magic `28 b5 2f fd` appears nowhere in any
block body, though every payload is now known to begin with a zstd frame — the
one place a plaintext byte sequence is predicted, it is absent. The `.img`
member proves the keying: both sample files carry the same 10-byte default
thumbnail, and their 63 stored bytes share **nothing** past the 8-byte constant
prefix. The nonce is random per file, so the cipher is keyed rather than a
fixed pad, and the 16-byte tag means a forged block would fail authentication
even if the cipher were known.

FM 2023's shortlists carry the identical `afe.` tag, the same `08 00 00` version
and the same `10 00 00 00 10 00 00 00` block prefix over a high-entropy body, so
this is not new in FM 26 and there is no older, plainer version of the format to
target instead.

### The version 07 sample, and a claim that did not survive

A `.fmf` from an older FM, kept off-machine, shows the format one version back.
It was originally recorded here as "a shortlist FM Genie Scout wrote", and that
attribution should be treated as unproven — see the correction below.

It is encrypted the same way. The file declares format version `07 00 00` and
its manifest is **zlib**, not zstd, but the block framing is identical and its
members are ciphertext:

| member | offset | stored | plaintext | difference |
|---|---|---|---|---|
| `100/.slf` | 0 | 193 | 153 | 40 |
| `shortlists/.jpg` | 193 | 21,272 | 21,232 | 40 |
| `_data/details.aom` | 21,465 | 137 | 97 | 40 |

The proof it is not plaintext is the `.jpg`. Its manifest declares 21,232 bytes
of JPEG and a JPEG must begin `ff d8 ff`, which appears **nowhere** in the
block, nor does `JFIF` or `Exif`. Version 07 has the same 40-byte crypto
overhead as version 08 and no compression layer, which is why the difference is
exactly 40 on a 97-byte and a 21,232-byte payload alike.

### Correction: "Genie Scout has SI's key" is not supported

An earlier draft concluded from this sample that a third-party tool writing
importable shortlists must be using SI's key. That does not hold up, and the
sample does not show what it was said to show.

The community position is that **Genie Scout never moved to `.fmf` at all**. It
still writes bare `.slf` files, and those have not imported into FM since the
game switched containers — renaming the extension does not help, and the
documented workaround is to route the list through FMRTE, which does write a
file FM accepts ([FM Scout: Shortlists .slf or
.fmf](https://www.fmscout.com/q-23134-Shortlists-slf-or-fmf.html), [Can't open
GS 22 shortlists in FM
2022](https://www.fmscout.com/q-24122-Can%C2%B4t-open-GS-22-short-lists-files-in-FM-2022.html),
[import workaround](https://www.fmscout.com/q-16589-Guide-HOW-TO-IMPORT-SHORTLISTS-WORKAROUND.html)).

So the version 07 archive above is far more likely to be an FM-written file
from 2019 — its timestamps are real Unix times from April 2019 — than
Genie output. Two things follow for anyone picking this up:

- **Do not cite Genie as evidence about the key either way.** It is evidence of
  nothing here; it does not write this container.
- **The encryption conclusion does not depend on it.** The FM 26 samples carry
  it on their own: random per-file nonce, 16-byte tag, and a zstd frame that is
  predicted at a known offset and is not there.

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
