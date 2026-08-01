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

## 2. String table — SOLVED, sectioned

Names are interned in a table of length-prefixed UTF-8:

```
u32  id
u32  byte length
[u8] UTF-8 bytes        (not null-terminated)
```

The table is not one id space. It is **sections whose id spaces overlap**,
each restarting from low ids: forenames first, then surnames, then a stray
single entry, then common names. In the reference save that is 291,140
forenames, 595,677 surnames and 92,812 common names, spanning `0x3202d46` to
`0x4125dfe` of the main frame. A flat id→string map resolves forename ids
against surname strings and produces names like "Maraga Ødegaard"; the section
must be known. Section identity is positional — of the sections big enough to
be name pools, the first is forenames, the second surnames, the third common
names — verified by Haaland's forename id 217140 resolving to "Erling" only in
the first and his surname id 434961 to "Haaland" only in the second.

Entries are grouped by origin within a section — Norwegian surnames sit
together, Spanish surnames together, and the same string can appear several
times under different ids. UTF-8 is genuine multi-byte (`c3 b8` = ø,
`c3 a5` = å), so decode properly rather than assuming Latin-1.

The table also fixes where person records start: they begin immediately after
it, which is what `strings.rs` exposes as `end_offset`.

## 3. Person record — SOLVED, both name layouts

The record prefix is three string-table references, each followed by a zero
byte, then the inline full name — **whose length is zero when the full name is
exactly "forename surname"** — then the date of birth:

```
u32  first_name_id        into the forename pool
u8   00
u32  surname_id           into the surname pool
u8   00
u32  common_name_id       0xFFFFFFFF when the player has no nickname
u8   00
u32  full_name_length     0 when the name is composed, else 2-64
[u8] full_name            present only when length > 0
u16  day_of_year          date of birth
u16  year
```

The zero-length case is the important one. FM stores an inline name only when
it differs from forename + surname — "Erling **Braut** Haaland" is stored,
"Virgil van Dijk" is not. A scanner that requires inline names sees 12,397
people; the save actually holds **49,217**, and the missing ~37,000 include
van Dijk, Declan Rice, Alexander Isak and every other plainly-named person.
The two cases cannot be confused: a length field's high half is zero while a
date's year half is 1920+, so the same four bytes never parse as both.

`common_name_id` is why a naive scan also misses players: Lamine Yamal and
Vinícius Júnior have nicknames, so the field is populated rather than
`0xFFFFFFFF`.

### The identity block — person entity ids, SOLVED

Several hundred bytes past the name, every person record carries an identity
block — the uid repeated beside the entity id, preceded by three zero bytes:

```
...  00 00 00
u32  eid                  entity id — what squad lists reference
u32  uid
u32  uid                  (repeated)
```

Confirmed values: Haaland eid 10241 / uid 29179241, Grealish 6961, Saka 8061,
van Dijk 11849. Entity ids ascend strictly through the person region — the
records are written in entity-id order — and that ordering is the acceptance
test: the same 15-byte shape recurs by chance in contract data, so the true
blocks are the longest strictly-ascending chain (patience LIS), which noise
does not survive. The flag byte six bytes before the eid varies (0x40 for most
people, 0x30/0x00/0x58 for newgens), so it cannot serve as the anchor.

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

The nation **names** are not stored beside the identifier. Every occurrence of
"England" in the database frame turns out to be the *surname* England, sitting
in the surname table between Boateng and Mullen — the country names are
probably in FM's localisation files rather than the save.

Twenty nations are named anyway, by grouping every person by nation identifier
and reading the surnames. A national squad is unmistakable: 143 gives Zidane,
Henry, Deschamps and Trézéguet; 158 gives Davids, Reiziger and van Nistelrooij;
33 gives Okocha, Kanu and Amokachi; 120 gives Donovan, Berhalter and
Cherundolo. Identifiers whose surnames are Spanish-speaking but not
country-specific are left unnamed and surface as raw numbers, which still group
and filter correctly.

## 4. Club record

Clubs carry a full name and the short name FM shows in tables. The header ends
with a three-byte signature immediately before the name length, and starts
with the club's own entity-id head:

```
u32  eid               entity id — what the squad table references
u32  uid
u32  uid               (repeated)
u8   00
u32  nation_id
u32  0xFFFFFFFF
u32  nation_id         (repeated)
u32  nation_id         (repeated)
u32  club_id
3    UNKNOWN
3    10 FF FF          signature
u32  name_length
[u8] name              e.g. "Manchester City"
u32  short_length
[u8] short_name        e.g. "Man City"
```

Confirmed entity ids: Arsenal 293, Liverpool 366, Manchester City 369,
Manchester United 370, Badalona 6610. Nation 139 for the English clubs, 145
for the German. 18,663 club-shaped records parse from the reference save, of
which 17,495 carry the validated head.

**`club_id` is not the club's identifier.** Manchester City and Manchester
United both carry 1075, Arsenal and Chelsea both 1040 — it looks like a city
or region id. Everything that references a club does so by `eid`. Beware also
that distinct club entities share display names: the reference save has two
"Manchester City" clubs, eid 369 (men) and eid 15524 (women), each with its
own squad.

The `10 FF FF` signature is doing real work. Two consecutive length-prefixed
strings is not a specific enough shape on its own — the file also contains the
commentary word lists, which produce thousands of pairs like
`("admirable", "amazing")` and `("bewitching", "brilliant")`. Requiring the
signature and an initial capital removes them.

### The eight u32s after the short name are not teams

Immediately after the short name sits `01`, six bytes, a `01`/`02` flag, a
count byte and that many u32s — for Manchester City eight values (6610, 6611,
7578, ...). These were long suspected to be the club's team list. They are
not: resolving them as club entity ids gives Badalona, Pittsburgh Riverhounds,
Stockport Georgians — unrelated small clubs worldwide. What that list actually
is remains open; it is not needed for squads.

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

The principled route turned out to be the squad table (section 6d): a player
is someone a club's squad record lists. The attribute-block rule of 6a ships
because it agrees with that table wherever both apply.

## 6. Attributes, Current Ability and Potential Ability — SOLVED

None of this is in the person record. FM stores a separate **attribute block**
and the ability values sit immediately in front of it.

```
block-39  u8   Current Ability     1-200
block-37  u8   Potential Ability   1-200, never below CA
block-15  15x  position ratings    raw 1-20
block+0   54x  attributes          internal 1-100; display = round(v / 5), floor 1
```

The block is exactly **54 bytes** on an internal 1-100 scale. On a freshly
started save every value is an exact multiple of 5 — the display value times
five — and that property was the original search signature. **It does not
survive play**: training and decline move internals off the multiples, and an
aged save (2035, ten seasons in) contains *no* multiples-of-5 blocks at all.
Musiala's Crossing there is stored as 66 and displayed 13; his Penalty Taking
as 68, displayed 14 — so the display conversion is round-to-nearest, not a
floor, confirmed against his in-game report on all twenty named attributes.

The durable signature is the whole structure at once: 54 bytes each 1-100,
the fifteen position bytes each 1-20 immediately before, and CA/PA in range
with PA >= CA at the fixed offsets in front. Searching for a run of raw 1-20
values finds nothing, because they are not stored that way.

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

## 6c. Attribute names — twenty identified, ground truth arrived

Two generations of evidence. The first fourteen came statistically, from two
signals that had to agree: which well-known players top the index, and how
the mean shifts by the player's strongest position. That method separated
Heading (3) from Jumping Reach (39) using goalkeepers — keepers jump
constantly and head almost never — and gave Marking (5) against Tackling (9)
directionally.

The second generation is **ground truth**: the in-game player report for
Jamal Musiala in an aged save, checked value-by-value against his decoded
block. Every statistically-named outfield index matched his screen exactly,
validating the whole first era, and the indices whose displayed value appears
exactly once on his screen are named outright:

| Index | Name | His value |
| --- | --- | --- |
| 26 | Flair | 19 |
| 27 | Corners | 10 |
| 29 | Work Rate | 15 |
| 34 | **Acceleration** | 14 |
| 35 | Free Kick Taking | 11 |
| 38 | **Pace** | 15 |
| 40 | Leadership | 9 |

The long-stuck 34/38 pair is settled — his screen shows Acceleration 14 and
Pace 15 and the pair reads {14, 15} — and the Marking/Tackling caveat is
closed (5 reads his 12, 9 his 11, as labelled). One correction: **index 40
was labelled Aggression and is Leadership** — his Aggression is 12 but index
40 reads 9, his exact Leadership. Otamendi, Ronaldo and Freuler topping it
fits captains as well as it ever fitted aggressors.

Still ambiguous, because several of his attributes share a displayed value:
{7, 22} is First Touch / Vision (both 17), {43, 53} is Bravery /
Concentration (both 13), {24, 45} holds Aggression and a hidden attribute
(both 12), and the 18/16/14 pools hold Dribbling, Composure, Agility,
Anticipation, Decisions, Determination, Balance, Long Shots, Teamwork,
Natural Fitness and Stamina among FM's hidden attributes. **One more in-game
report of a player with different values breaks most of these ties** — that
is the cheapest remaining move.

Index 25 is not an attribute: it reads exactly 100 in every real block
observed, fresh or aged — a constant, and a useful sanity check.

Earlier mistake worth keeping: index 6 was first labelled Heading because
Ronaldo, Haaland and Mitrović top it — but centre-backs average 8.5 there
against strikers' 12.0, and centre-backs head constantly. It is Off the Ball.
Player-topping evidence alone is never sufficient.

## 6a. Players vs staff — SOLVED

Only players have a 54-byte attribute block. Staff either have none or a shorter
run. That absence is the discriminator, and it is structural rather than the
statistical guess rejected in section 5. In the reference save: **3,999 players,
8,398 staff**.

## 6d. Squad membership — SOLVED, a separate table

A club is linked to its players through a dedicated **squad table**, one
record per club, ordered by club entity id. In the reference save it spans
`0x1ed2ece` to `0x205b642` of the main frame — 14,047 records, of which 1,814
have a squad:

```
u32   club_eid           matches the club record's head
10x   00
u32   UNKNOWN            (eid + 131 in the observed range)
u32   club_uid           matches the club record's head
u32   club_uid           (repeated)
...   variable fields
u32   0xFFFFFFFF
u16   count              squad size, 1-40s
u32[] person_eids        the squad, matching person identity blocks
u32   captain_eid        0xFFFFFFFF when unset
u32   vice_captain_eid   0xFFFFFFFF when unset
...   trailing fields
```

Two properties make the walk trustworthy. A head is only accepted when its
`(eid, uid)` pair **matches what the club table carries** for the same entity
id — two independent tables agreeing. And the list itself must look like a
squad: on a fresh save it ascends by entity id with new signings appended at
the tail, but **a decade of transfers destroys that order entirely** — 2035
Liverpool's list opens 24359, 15005, 10164. What survives ageing is that the
captain and vice-captain following the list are still members of it, so the
parser accepts either signal: ascending, or captain-linked.

Verified against reality on the reference save (October 2025, so with the
2025 summer window applied): Manchester City's 33 include Haaland, Grealish,
Foden, Donnarumma, Cherki and Marmoush with Bernardo Silva as captain and
Rúben Dias vice; Liverpool's 32 include Wirtz, Isak, Ekitiké, Kerkez and
Mamardashvili with **Virgil van Dijk** captain; Arsenal's captain resolves to
Martin Ødegaard and Manchester United's to Bruno Fernandes. Kyle Walker —
sold in 2025 — is correctly absent from City.

Resolution rate: 15,520 of the 15,558 distinct person eids referenced across
all squads resolve to a person record (99.76%). The 38 misses are recorded in
`OPEN_PROBLEMS.md`.

The four approaches ruled out on the way (a club id inside the person record,
rare shared values, same-club agreement sweeps, id arrays in the club body)
are preserved in the git history of this file; the root mistake was assuming
person records had no identifier — they do, the identity block of section 3.

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
- `clubtable.py`, `clubbody.py` — club entity heads and the eight-u32 dead end
- `squadhunt.py`, `eidanchor.py`, `validated_walk.py` — finding the squad table
- `fullscan2.py`, `pipeline_v2.py` — the person scan v2 and its quality
  measurements, ending at 99.76% squad resolution

They are throwaway spikes kept for provenance, not part of the build.
