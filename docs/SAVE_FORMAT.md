# Football Manager 26 save format

Reverse-engineered from `Career.fm` (FM 26.0.0, macOS). Everything here was
verified against a real save; anything unverified is marked **UNKNOWN**.

Sample file: 44,673,908 bytes → 187,817,701 bytes decompressed (4.2x).

## 1. Container

```
offset  size  meaning
0       2     02 01            version? (constant across saves seen so far)
2       4     "fmf."           magic
6       20    header           includes a u32 at +9 close to the file size
26      ...   zstd frame       first of N back-to-back frames
```

From byte 26 to EOF the file is a sequence of Zstandard frames laid end to end.
There is no frame index — you decode a frame, ask the decoder how many bytes it
consumed, and that lands you on the next frame's magic (`28 b5 2f fd`).

Two traps:

- `zstd -d` on the whole tail fails with `unsupported format`, and the Python
  `stream_reader` fails with `Unknown frame descriptor`. Neither is a data
  problem. Use a one-frame-at-a-time decompressor that reports unused input
  (Python `ZstdDecompressor().decompressobj()`, Rust `zstd::Decoder` +
  `total_in`).
- Searching for the magic to find frame boundaries gives false positives —
  the byte sequence occurs inside compressed payloads too. The sample has 1,760
  magic occurrences but only **1,215 real frames**. Always advance by bytes
  consumed, and only scan for the next magic when recovering from an error.

Every decompressed frame starts with `03 01` + `"tad."`. Frame 0 also carries
the version string `26.0.0+0`.

Frame 3 is the main database: 29.7 MB compressed, **105.7 MB decompressed**.
Everything below lives in frame 3.

## 2. String table

Names are interned in a table of length-prefixed UTF-8, IDs ascending:

```
u32  id
u32  byte length
[u8] UTF-8 bytes        (not null-terminated)
```

Entries are grouped by origin — Norwegian surnames sit together, Spanish
surnames together, and so on. UTF-8 is genuine multi-byte (`c3 b8` = ø,
`c3 a5` = å), so decode properly rather than assuming Latin-1.

## 3. Person record

Located by searching for a surname ID reference. For Haaland the surname
`"Haaland"` interns as ID `0x0006A311`; that value appears 3 times in frame 3 —
once in a lookup array, once in the string table itself, once in the person
record.

Layout, ending with an inline full name:

```
u32  surname_id           into the string table
u8   UNKNOWN
u32  common_name_id       0xFFFFFFFF when the player has no nickname
u8   UNKNOWN
u32  full_name_length     bytes, not chars
[u8] full_name            UTF-8, e.g. "Erling Braut Haaland"
```

`common_name_id` is why a naive scan misses players: Lamine Yamal and Vinícius
Júnior have nicknames, so the field is populated rather than `0xFFFFFFFF`. A
scanner that hard-codes `ff ff ff ff` silently drops every player with a
nickname — 12,023 records were found this way while known players were absent.

### Fields after the name — verified

Offsets are relative to the **end of the full name**.

| Offset | Type | Meaning |
| --- | --- | --- |
| +0 | u16 | **Day of year** of birth (1-366) |
| +2 | u16 | **Year** of birth |

Date of birth is stored as day-of-year plus year, not day/month/year. Confirmed
exactly on five players:

| Player | Real DOB | +0 | +2 |
| --- | --- | --- | --- |
| Erling Braut Haaland | 21 Jul 2000 | 203 | 2000 |
| Jude Victor William Bellingham | 29 Jun 2003 | 180 | 2003 |
| Florian Richard Wirtz | 3 May 2003 | 123 | 2003 |
| Bukayo Ayoyinka Saka | 5 Sep 2001 | 248 | 2001 |
| Kylian Mbappé Lottin | 20 Dec 1998 | 354 | 1998 |

Day 203 of a leap year is 21 July; day 123 of 2003 is 3 May. Across all 12,023
records, 94% of the +2 values fall in 1970–2012, and the distribution peaks at
1995–2006 — the shape a football database should have.

### Fields after the name — unresolved

`+13` is a single byte that survives the obvious sanity filters (99% within
1–200, few zeros, 173 distinct values) but is **not** confirmed as CA:

| Player | +13 |
| --- | --- |
| Haaland | 160 |
| Wirtz | 145 |
| Mbappé | 143 |
| Bellingham | 139 |
| Saka | 139 |

Two reasons to doubt it. The population mean is 146 with an p10–p90 band of
115–175, far too high and too narrow for Current Ability, which should average
around 70–100 across a full database. And the ordering is wrong — Mbappé below
Haaland by a wide margin does not match FM's own ratings.

Worth noting for whatever this turns out to be: FM stores the 1–20 attributes
internally on a 1–200 scale and divides by 10 for display, so a byte of 160
displays as 16. That makes `+13` plausibly a single displayed attribute rather
than CA.

## 4. CA/PA — not yet located

The record body past roughly +14 is **variable length**: it contains repeated
~16-byte sub-blocks (visible as recurring `xx xx 00 03 01 32 4f 00 ff` style
rows). Fixed offsets from the name therefore stop being meaningful, which is why
a column-wise scan for a CA/PA pair over the first 160 bytes found nothing.

The structural test to apply once the sub-blocks are parsed: **PA ≥ CA for every
player**, both within 1–200. Across ~12k records that constraint is strong
enough to identify the pair on its own, with no ground truth needed.

Approaches not yet tried:

- Parse the repeating sub-blocks properly and treat each as a typed key/value,
  rather than assuming a flat struct.
- Diff two saves of the same career a few in-game months apart. CA moves for
  developing players while DOB, height and IDs stay fixed, so the diff narrows
  the search enormously.
- Cross-check against in-game values via the FM26 in-game editor for a handful
  of players to confirm a candidate offset.

## 5. Prior art

FM Scouting Tool 26 (the Electron app on fmscout.com) does **not** parse saves.
Its `app.asar` bundles `koffi` and calls `OpenProcess` / `ReadProcessMemory`
from `kernel32.dll` against the running FM process, reading `game_plugin.dll`.
That approach does not port to macOS — `task_for_pid` needs root or a debugger
entitlement — which is the reason this project parses the save file instead.

`robeady/fm-explorer` similarly reads process memory via FMScoutFramework and is
Windows-x64 only.

## 6. Reproducing

`research/` holds the Python spikes used to derive the above:

- `unpack.py` — splits a `.fm` into `frames/fNNNN.bin`
- `findplayers.py` — scans frame 3 for person records
- `findca.py` — column statistics over post-name bytes

They are throwaway spikes kept for provenance, not part of the build.
