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

### `+13` is nationality — SOLVED

`+13` is a `u16` nation identifier, not an ability value. It was flagged early
as a CA candidate and rejected on three counts: the population mean of 146 was
far too high, its maximum of 247 exceeds the 200 ceiling, and it never changed
between saves. All three are explained by nationality — it is an identifier,
not a rating, and a player's nation does not change.

Confirmed values, cross-checked against the nation the **club** records carry,
which uses the same numbering:

| ID | Nation | Confirmed from |
| --- | --- | --- |
| 139 | England | Saka, Kane, Walker, Grealish, Bellingham; the English clubs |
| 143 | France | Mbappé |
| 145 | Germany | Wirtz; Borussia Dortmund |
| 160 | Norway | Haaland |
| 162 | Portugal | Rúben Dias and Ronaldo, independently |
| 187 | Argentina | Messi |
| 189 | Brazil | Alisson |

The nation **names** are not stored beside the identifier — searching around
each name string finds no nearby copy of its ID — so only the verified set is
named. Unconfirmed IDs surface as raw numbers, which still group and filter
correctly.

## 4. Club record

Clubs carry a full name and the short name FM shows in tables. The header ends
with a three-byte signature immediately before the name length:

```
u32  0xFFFFFFFF
u32  nation_id
u32  nation_id        (repeated)
u32  club_id
3    UNKNOWN
3    10 FF FF          signature
u32  name_length
[u8] name              e.g. "Manchester City"
u32  short_length
[u8] short_name        e.g. "Man City"
```

Confirmed values: Manchester City `club_id` 1075, Arsenal 1040, Borussia
Dortmund 541. Nation 139 for the English clubs, 145 for the German, 126 for the
Albanian. 18,663 clubs parse from the reference save.

The `10 FF FF` signature is doing real work. Two consecutive length-prefixed
strings is not a specific enough shape on its own — the file also contains the
commentary word lists, which produce thousands of pairs like
`("admirable", "amazing")` and `("bewitching", "brilliant")`. Requiring the
signature and an initial capital removes them.

Nation IDs are **not** yet resolved to names, and squad lists are not located,
so a club cannot yet be linked to its players.

## 5. Players vs staff — a rejected approach (superseded by 6a)

Players and staff share the same record layout; Maldini and Davids parse
identically in shape to Haaland and Saka. There is a real difference — staff
records carry two null `u32`s where a player has a value followed by a contract
date (day-of-year 60, year 2023 for Haaland, against the null 1 January 1900
for staff) — and a `u32` read 24 bytes before the record start splits the
population 47/53 and agrees with all eight hand-labelled knowns, with mean birth
year 1998 against 1990.

It is still not shipped as a player/staff flag. Reading the actual bytes shows
that offset straddles field boundaries rather than landing on one: the section
before the name is variable-length, so the agreement is luck rather than
structure, and it would break on the first record shaped differently.

The principled route is the club record's squad list — a player is someone a
club lists as a player. That needs the club record body parsed, which is not
done.

## 6. Attributes, Current Ability and Potential Ability — SOLVED

None of this is in the person record. FM stores a separate **attribute block**
and the ability values sit immediately in front of it.

```
block-39  u8   Current Ability     1-200
block-37  u8   Potential Ability   1-200, never below CA
block+0   54x  attributes          each is the 1-20 value multiplied by 5
```

The block is exactly **54 bytes**, every one a multiple of 5 in 5..=100, which
is FM's 1-20 attribute scale stored at 5x (100 displays as 20, 65 as 13). That
multiple-of-5 property is the signature that finds it; searching for a run of
raw 1-20 values finds nothing, because they are not stored that way.

**The block precedes the person it belongs to**, by a median of about 1,200
bytes. So the owner is the next person record after the block — not the other
way round. Pairing a block to the *preceding* name attributes ability to the
wrong player and produces plausible-looking nonsense (Haaland at CA 105).

Verified against real ratings:

| Player | CA | PA |
| --- | --- | --- |
| Kylian Mbappé | 191 | 197 |
| Erling Braut Haaland | 184 | 195 |
| Jude Bellingham | 181 | 188 |
| Bukayo Saka | 181 | 188 |
| Florian Wirtz | 170 | 188 |
| Lionel Messi (39) | 172 | 200 |
| Cristiano Ronaldo (41) | 155 | 195 |

Messi at PA 200 with CA well below it, and Ronaldo down to 155 at 41, are
exactly what a declining great looks like. Across 5,847 blocks: CA averages
102.7 (median 105, max 191), PA averages 121.0 and tops out at exactly 200, and
**PA >= CA holds for 99.98%**.

How it was found, since the method generalises: CA is a weighted function of the
attributes, so once the block gave a reliable anchor, sweeping every nearby
offset and correlating against the attribute mean identified CA at r = **0.938**.
No ground truth was needed. Anchoring on the name instead never works — the
section between name and block is variable length.

Two traps worth recording. Distances from block to owner are far more variable
than they first look: median ~1,200 bytes but the 99th percentile is ~29,000, so
a tight cap silently drops thousands of players. And several blocks can resolve
to the same person because the person scan misses records; the nearest block
must win, or a distant one overwrites a correct match and quietly moves ability
between players.

## 6b. Positions — SOLVED

The 15 bytes immediately **before** the attribute block are position ratings,
each 1-20 for how naturally the player plays there. Unlike the attributes these
are not scaled, so they read directly.

Slot order, established from players whose real position is unambiguous:

| Slot | Position | Evidence |
| --- | --- | --- |
| 0 | GK | Alisson 20, everything else 1 |
| 1 | SW | nobody's strongest position, as expected in a modern database |
| 2 | DL | |
| 3 | DC | Rúben Dias 20; the most common strongest position at 19.7% |
| 4 | DR | Kyle Walker 20 |
| 5 | DM | |
| 6 | ML | Grealish 18 |
| 7 | MC | Bruno Fernandes 18; 14.3% of players |
| 8 | MR | |
| 9 | AML | Grealish 20, Mbappé 20 |
| 10 | AMC | Bruno Fernandes 20, Grealish 20 |
| 11 | AMR | Saka 20, Messi |
| 12 | ST | Haaland 20, Kane 20, Ronaldo |
| 13 | WBL | |
| 14 | WBR | Walker 16, matching his wing-back role |

Cross-checks that would have failed on a wrong ordering: Haaland and Ronaldo
resolve to ST alone, Saka to AMR/AML, Mbappé to AML/ST/AMR, Bellingham to
MC/AMC/ML, Messi to AMR/AMC/ST, and Naomi Girma to DC. The population shape is
right too — centre-back most common, sweeper unused.

## 6c. Attribute names — partly identified

Ten of the 54 are named, from two signals that must agree: which well-known
players top the index, and how the mean shifts by the player's strongest
position. Crossing, Finishing, Off the Ball, Penalty Taking, Passing,
Positioning, Technique, Long Throws, Strength, Aggression.

The position data caught a mistake worth recording. Index 6 was first labelled
Heading because Ronaldo, Haaland and Mitrović top it — but once positions were
decoded, centre-backs average 8.5 there against strikers' 12.0. Centre-backs
head the ball constantly, so Heading was wrong; the shape is an attacking
movement attribute instead.

Some indices form matched pairs with identical positional signatures and cannot
be separated: Marking against Tackling (5 and 9), Pace against Acceleration
(34 and 38), Heading against Jumping Reach (3 and 39). These stay unnamed.

Index 25 is an oddity — mean 17.3 with most players at or near 20 — which is
not the shape of any 1-20 attribute, so it may not be one.

## 6a. Players vs staff — SOLVED

Only players have a 54-byte attribute block. Staff either have none or a shorter
run. That absence is the discriminator, and it is structural rather than the
statistical guess rejected in section 5. In the reference save: **3,999 players,
8,398 staff**.

## 7. Prior art

FM Scouting Tool 26 (the Electron app on fmscout.com) does **not** parse saves.
Its `app.asar` bundles `koffi` and calls `OpenProcess` / `ReadProcessMemory`
from `kernel32.dll` against the running FM process, reading `game_plugin.dll`.
That approach does not port to macOS — `task_for_pid` needs root or a debugger
entitlement — which is the reason this project parses the save file instead.

`robeady/fm-explorer` similarly reads process memory via FMScoutFramework and is
Windows-x64 only.

## 8. Reproducing

`research/` holds the Python spikes used to derive the above:

- `unpack.py` — splits a `.fm` into `frames/fNNNN.bin`
- `findplayers.py` — scans frame 3 for person records
- `findca.py` — column statistics over post-name bytes

They are throwaway spikes kept for provenance, not part of the build.
