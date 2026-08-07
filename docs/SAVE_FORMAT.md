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

Every decompressed frame starts with `03 01` + `"tad."` — except the last,
which is the member manifest (§1b). Frame 0 also carries the version string
`26.0.0+0`.

Frame 3 is the main database — the manifest names it `game_db.dat`: 29.7 MB
compressed, **105.7 MB decompressed**. Everything below (§§2–6) lives in
frame 3.

## 1b. The member manifest — every frame has a name, SOLVED

The save is not an anonymous frame stream. It is an **archive**, and the last
frame is its manifest — the same member layout as the `.fmf` shortlist archive
(`SHORTLIST_FORMAT.md` §3), which is how it was recognised. The manifest frame
has no `03 01 "tad."` prefix; decompressed it reads:

```
u32 len + bytes     save name ("Career")
u32                 top-level member count
per member:
  u32 len + bytes   name parts, repeated until one begins with '.'
  u64               offset, relative to byte 26
  u64               stored length == the member's compressed frame size
  u64               plaintext length == the member's decompressed size
  8 bytes           stamp   (paired values consistent with unix timestamps)
  8 bytes           stamp
u32                 sub-archive count
per sub-archive:
  u32 len + bytes   name ("rgman")
  u32               child count
  children          same member record as above
00 00 00 00         terminator
```

Members sorted by offset are exactly the frames in file order: the reference
save declares 77 top-level members plus one sub-archive `rgman` of 1,137
competition rule groups (`comp_<uid>.dat`), 1,214 in all — frames 0–1213, with
the manifest itself as frame 1214. Offsets tile the file from 0 with no gaps,
and all 1,214 plaintext lengths match the decompressed frames byte for byte.
Verified identically against a 188 MB `Ongoing.fm` (106 members) and a fresh
career's save.

Two things follow:

- **Random access.** `offset + 26` is a file position; one seek and one zstd
  frame decode extracts any member. No need to decompress the save up to it.
  `research/members.py` lists and extracts by name.
- **A map of everything not yet decoded.** `shortlist_man.dat`,
  `tactics_man.dat`, `injury_manager.dat`, `transfer_man.dat`,
  `humans.dat` — each subsystem is a named member, so future work starts from
  a labelled 1–2 MB frame instead of a 106 MB haystack.

Named members confirmed so far: `game_info.dat` (frame 0), `memory_pools.dat`,
`save_game_summary.dat`, `game_db.dat` (frame 3), `humans.dat`,
`shortlist_man.dat`, `manager_manager.dat`, `scout_man.dat`, and ~70 more —
run `members.py` for the full list of a given save.

## 1c. The in-game date — SOLVED, one reader for both format versions

Dates throughout the save are `(u16 day_of_year, u16 year)` pairs, the same
encoding as a date of birth. The current date is the **week stamp at the head
of `game_db.dat`, offset 0x2A**, on 26.0.0 and 26.2.0 alike: a u16 whose **low
nine bits are the day of year** (the high seven vary per save and are not
understood — 0, 13, 25, 27 and 41 observed), then the u16 year. It tracks the
weekly rollover, so it lags the true date by up to a week.

Verified on seven saves. The 2035 career masks to 24 May 2035 against a known
true 28 May; the Afan Lido career masks to day 159 of 2026 (8 June), exactly
the current-date stamp repeated through that save's `rgman/comp_*.dat`
competition frames; a save named "Day One" reads 23 June 2025, FM 26's first
day. On 26.0.0 the same offset reads 20 September 2025 for a young career and
1 July 2033 for one eight years in.

**The header frame's stamp is the real-world time the file was written, not
the in-game date.** `game_info.dat` offset 50 carries the same masked
`(stamp, year)` shape, and on the four 26.2.0 careers here it reads 1, 2 and 3
August 2026 — those files' own modification dates — while the careers sit in
2026, 2030, 2032 and 2035. On 26.0.0 saves it reads late October 2025, the
real-world month 26.0.0 was the shipping format. `find_wall_clock_date` reads
it under that name; nothing about anyone's age may come from it.

**Scar.** This was recorded the other way round for a while: the header pair
was taken as the 26.0.0 in-game date, and the week stamp gated to 26.2.0-only
because on 26.0.0 it "masked to a valid-looking wrong date" (day 199 against a
true 26 October). Both halves came from the same mistake. The wrong date came
from masking the **largest** frame rather than `game_db.dat` — on a long career
`player_stats_hist_dt.cmt` outgrows the database (372 MB against 351 MB), which
is the same trap §1b's `main_frame` note records. And the header pair only
looked right because the reference save was written weeks after the in-game
date it sat at, so wall clock and game date fell in the same season. An aged
26.0.0 save shows it plainly: the header says October 2025, the database says
July 2033, and the save's sixteen-year-old newgens were being reported as
eight-year-olds.

Diagnostics that read the date must name `game_db.dat` through the manifest,
never take the largest frame — `datescan` and `diagnose` do now.

The save-list summary (`save_game_summary.dat`) repeats the same stamp pair
after the manager's name and club, so it is a stale copy too, not an exact
source. The exact current date does exist per-save in the `rgman/comp_*.dat`
members, but which competitions carry it varies by career, so nothing is read
from there.

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

228 nations are named anyway. The first 150 came from grouping every person by
nation identifier and reading the best players: a national squad is
unmistakable — 143 gives Zidane, Henry, Deschamps and Trézéguet; 158 gives
Davids, Reiziger and van Nistelrooij; 33 gives Okocha, Kanu and Amokachi; 120
gives Donovan, Berhalter and Cherundolo.

**The other 78 came from the clubs, which is the better method and should have
been first.** Club records carry the same numbering and store their names *in
the clear*. "Ba FC, Labasa FC, Lautoka FC, Nadi FC" is Fiji and nothing else,
where a squad of Fijian-sounding players only suggests it; "Al-Sadd, Al-Wakrah,
Al-Shamal" is Qatar; "Bolívar, The Strongest, Wilstermann" is Bolivia. One pass
over a save with every league loaded named 78 identifiers at once, most of them
the small federations a squad-reading pass could never settle — Kiribati,
Wallis and Futuna, Saint Pierre and Miquelon, Zanzibar. `examples/nations.rs`
prints clubs beside people for exactly this.

The identifiers are broadly regionally alphabetical, on FM's own older names
(Africa 0–50 with 44 The Congo and 48 Zaire; Asia 51–91 with 63 Kampuchea and
86 The Philippines; CONCACAF 92–125 with 118 The Bahamas; UEFA 126–176; later
admissions at 200+), which double-checks every identification but does not
override it — 114 Saint Lucia sits ahead of 115 Saint Kitts and Nevis.

Reading the clubs also **corrected an entry the player-name method got wrong**:
116 was Cayman Islands here, but its clubs are Avenues United, Layou FC and
North Leeward Predators — Saint Vincent and the Grenadines. Cayman is 98, where
Bodden Town FC and Scholars International play.

Three groups are left unnamed and surface as raw numbers, which still group and
filter correctly: 213 is four British Army regimental sides, 123 a single East
German club with no people, and 238 three people with no clubs at all
(`OPEN_PROBLEMS.md` §5).

### An identifier past the end of the nation table means it is not a person

The highest identifier FM uses is 249, Saint Barthélemy. Records that read past
it read far past it — 1280, 8704, 45209 — because they are not records: they
are other tables' bytes that happen to satisfy the person prefix (three
resolving string ids, then something that decodes as a date of birth). A frame
of 350 MB gives coincidence plenty of room.

The scan therefore refuses a record whose nation field reads above 512, chosen
well clear of both 249 and 1280 so a nation FM adds later is not mistaken for
noise. An *absent* nation field still passes — a record truncated at the end of
the frame has nothing to judge.

The cost and the benefit, measured on two saves: a day-one save loses 1,043
records, none of which has a club, four of which have an entity id, and five of
which had captured an ability block that belonged to a real player nearby. An
eight-year career loses 796. What goes with them is every person whose date of
birth put them outside a footballing lifetime — 689 under-fourteens and a
109-year-old on the aged save, all of them these coincidences wearing
cross-cultural mashup names like "Oldřich Dharmaraja Singgam". One survivor
remains out of 239,403, and it stays: a tighter bound would start costing real
people, and the save's genuinely old are real — Étienne Davignon, born 1932, is
Anderlecht's honorary chairman.

### The hidden personality run — SOLVED, four slots named

Between the date of birth and the identity block, every adult record repeats
its nation identifier and follows it with the eight hidden personality
attributes:

```
u8   citizenship count (varies; not part of the match)
u16  nation_id           repeated — this is the match's anchor
6x   00
8x   personality         each 1-20
```

All eight slots are named, in storage order: **0 Adaptability, 1 Ambition,
2 Loyalty, 3 Pressure, 4 Professionalism, 5 Sportsmanship, 6 Temperament,
7 Controversy**. Slots 0, 4 and 7 came from in-game staff screens (Elite
reads 20, a "Model Professional" 20, Controversy near-universally low);
the rest fell on 3 August 2026 to the pre-game editor, whose sheet for
Guardiola — Adaptability 20, Ambition 20, Loyalty 15, Pressure 18,
Professionalism 20, Sportsmanship 16, Temperament 14, Controversy 8 —
matches his run `20 20 15 18 20 16 14 8` exactly, with every ambiguous
value distinct. (Slot 1 was earlier misread as Loyalty from a screen where
both were high; the editor separates them.)

Coverage is every adult with a real nation; the misses are the *children*
an aged save simulates into existence (different layout, junk nation ids)
and human-manager avatars.

### The second object — where staff data lives

A person owns **two** entity objects: the person object (what squads
reference) and a non-player object. Their **uids are persistent DB ids**
(Haaland's player object is uid 29179241 — the same id sortitoutsi and
fmscout use in their URLs — and his non-player object 29179299 in every
save), while **eids are per-save indexes** that renumber between saves
(that non-player object is eid 10242 in Career.fm, 12400 in the 2035 save).

The non-player object is *inside* the person record only sometimes (~1,100
of 49,217 records in Career.fm, `objmap` example); usually it lives
elsewhere and the record carries 30-byte references instead:
`10 00 [u32][u32] [flags] 04 00 00 00 [eid][uid][uid]`, zero to two of
them directly after the person's identity block on aged saves. Where the
in-record object does occur, its identity block is followed by `01/02`,
three u16s, a pair, and — laid out exactly like a player's — **a second
54-byte attribute block** on the 1-100 scale, index 25 = 100, with its own
15 position bytes. The scanner never saw these because the CA/PA offsets in
front land in an `FF` run, which the block test rejects.

Directly after the person's own identity block (past any `10 00`
references) sits a `02 [u16 A][u16 B][u16 C] ...` run of live state. It is
**not** the reputation triple, despite its shape: published FM 26
reputations order Haaland 96 > Chevalier 70 while the run reads Chevalier
6400 > Haaland 5250 in the same save. Its semantics are open —
`OPEN_PROBLEMS.md` §3b has the full evidence.

For pure staff (Emery), the record carries an id→value row list
(`[u32 id] … [value 1-100] [4f|00] ff` rows) holding coaching badges and
nation knowledge — Emery's England and Spain rows read 100, matching his
"Complete" knowledge — and then, after the person object's own
`02 40 10`-headed identity block at the record tail, **a 54-value attribute
block of its own**: tag byte `01`, five u16s (5000/5000/1500/125/125 for
Emery), then 62 bytes whose last 54 end at the record's `8×FF` terminator.
Anatomy of the 54 bytes, from 942 extracted blocks across a 26.0.0 and a
26.2.0 save (`staffmap` example): slot 1 is a constant 12 (structure, not
data); slots 2-27 (except 13 and 15) are twenty-four 1-20 values; slots
30-42 are a **stable thirteen-slot 1-100 array** that survives across
saves per person (±drift); slots 43-53 are a **mutable tail whose values
reshuffle between saves** — a list-like region, not a fixed array. Slots
0, 15, 28 and 29 are oddballs on their own ranges.

The block holds the pre-game editor's attribute list as the editor's
**controlled** (effective in-game) values, stored controlled×4 on 1-80
for CA-weighted attributes and raw 1-20 for weighting-0 ones — with a
small per-save rebase on top. The five u16s between the identity and
the block are `[A][B][C][D][E]` with **D ≡ B/50** and A/B/C multiples
of 50 — three 0-10000 values plus the raw 0-200 B and a second 0-200
value. The same structure follows *player* identity blocks. Its
semantics are **open**: for staff the triple shape-matches the editor's
"Game Reputations" line, but on players the reputation reading is
contradicted outright (Wissa 9300 vs Haaland 5250 on a day-one save),
so no reputation label ships from this field. Only **slots 30-42 of the
block are a stable DB-derived array** (byte-identical for the same
person across different careers); the 1-20 region and slots 43-53
**reorder per save** — a serialized list, not a fixed array. Cross-save
locks: slot 36 = Coaching Possession (controlled-10 → 40 signature
exact), 34 ≈ Negotiating, 15 ≈ Motivating, 1 ≡ 12. One historical
negative stands: efem.club's numbers are editor×5, not the save's ×4 —
fitting against them can never converge. `OPEN_PROBLEMS.md` §3b has the
full evidence and the tooling.

## 4. Club record

Clubs carry a full name and the short name FM shows in tables. The header ends
with three per-club bytes immediately before the name length — long read as a
flags byte and a fixed `FF FF` signature, none of the three constant — and
starts with the club's own entity-id head:

```
u32  eid               entity id — what the squad table references
u32  uid
u32  uid               (repeated)
u8   00
u32  nation_id
u32  0xFFFFFFFF
u32  location_nation   where the club sits — NOT a repeat, see below
u32  nation_id         (repeated)
u32  club_id
3    UNKNOWN
u8   flags             usually 0x10; 0x00/0x01/0x11/0x12/0x14/0x30 also occur
2    UNKNOWN           usually FF FF; FF 00, 00 00 and 00 FF also occur
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

### None of the three bytes before the name is a signature

The byte before `FF FF` is a **per-club flags byte, not part of the
signature**. An Afan Lido save reads 0x10 on 17,203 records but 0x00 on 522,
0x11 on 299, 0x12 on 6 (Tottenham and Chelsea among them), 0x14 on 3 and 0x30
on 13. Anchoring the scan on `10 FF FF` silently dropped every non-0x10 club —
and squads validate against the club table, so those clubs' entire squads
vanished with them and their contracted first-teamers showed no club.

**The `FF FF` behind it is per-club as well** (6 August 2026). Anchoring on
that pair had the same failure, one step along: the user's own club, Heybridge
Swifts, reads `10 FF 00` and so could not be found in Gilet at all. Surveying
every position in a 26.2.0 save whose *entity head* validates and whose tail is
a plausible name/short-name pair gives 19,365 clubs, and the three bytes
distribute:

| bytes | count |
|---|---|
| `10 FF FF` | 17,777 |
| `00 FF FF` | 721 |
| `10 FF 00` | 419 |
| `11 FF FF` | 257 |
| `10 00 00` | 114 |
| `11 FF 00` | 51 |
| `30 FF FF` | 16 |
| `10 00 FF` | 7 |
| `01 FF FF` | 2 |
| `11 00 00` | 1 |

592 of those clubs sit behind a pair that is not `FF FF` — Heybridge Swifts,
Newport County, Birmingham City, Blackburn Rovers, Bolton Wanderers, Boston
United among them. The reader therefore anchors on the **entity head**, not on
those bytes at all: a record whose head validates is a club whatever the three
bytes read. A record with no validating head must still carry the
long-verified `10 FF FF`, because two consecutive length-prefixed strings is
not a specific enough shape on its own — the file also contains the commentary
word lists, which produce thousands of pairs like `("admirable", "amazing")`
and `("bewitching", "brilliant")`. Requiring an initial capital removes those
too.

The head check has to run before the strings, not after: parsing two
length-prefixed strings at every offset of a 285 MB frame takes over a minute,
where testing the head's `FFFFFFFF` first keeps the whole parse at 3.5 s.

On the Heybridge save the fix is worth **+592 clubs, +417 squads and +5,505
players linked to a club**; on a day-one save +97 clubs and +39 linked players,
and no club entity id is claimed by two records in any save tested.
`crates/fm-save/examples/clubtail.rs` reproduces the survey and
`crates/fm-save/examples/linkstats.rs` the before/after numbers; the regression
test is `a_club_whose_tail_pair_is_not_ffff_keeps_its_squad`.

### The third nation u32 is where the club sits, not a repeat

The head carries the nation id three times — except it does not. The copy at
−18 from the name length is a **separate value: the country the club is
physically in**, where the two at −14 and −26 are the pyramid it plays in. On
99.8% of clubs those are the same country, which is why it read as a third
repeat for so long.

Requiring all three to match dropped the entity head of every cross-border
club, and a club with no `eid` cannot be referenced by a squad record — so
their whole first team showed no club. On a day-one save 39 clubs broke on
exactly this and nothing else, and the list is a roll-call of the real
cross-border cases:

| club | plays in | sits in |
|---|---|---|
| The New Saints | Wales 175 | England 139 |
| Cardiff City, Swansea City, Wrexham, Newport County | England 139 | Wales 175 |
| Derry City | Ireland 163 | Northern Ireland 159 |
| Berwick Rangers, Tweedmouth Rangers | Scotland 167 | England 139 |
| F.C. Andorra | Spain 170 | Andorra 127 |
| Wellington Phoenix | Australia 177 | New Zealand 180 |
| Bishops Castle Town | Wales 175 | England 139 |
| six Zanzibar clubs | 239 | Tanzania 43 |

The reader now requires the two copies at −14 and −26 to agree and bounds the
location field to a plausible nation id rather than demanding it match. On the
day-one save that took people linked to a club from 27,458 to 27,652 and clubs
owning a squad from 3,327 to 3,336. `crates/fm-save/examples/clubgap.rs`
reproduces the survey; the regression test is
`a_cross_border_club_resolves_its_squad` in `tests/real_save.rs`, which pins
Jack Sion to The New Saints.

### The eight u32s after the short name are not teams — they are the board

Immediately after the short name sits `01`, six bytes, a `01`/`02` flag, a
count byte and that many u32s — for Manchester City eight values (6610, 6611,
7578, ...). These were long suspected to be the club's team list. They are
not: resolving them as club entity ids gives Badalona, Pittsburgh Riverhounds,
Stockport Georgians — unrelated small clubs worldwide.

**Resolved 7 August 2026: they are person entity ids — the boardroom.** The
shape is `01 [u16] [u32 director-of-football] [flag] [count] [count x u32
board members] 01`, and on the 2035 index save the names check out against
reality: Jean-Louis Leca as Lens' sporting director with owner Joseph
Oughourlian on the board, Hugo Viana at Manchester City, Richard Hughes at
Liverpool, Andrew Cavenagh chairing Rangers. Only ~700 of 20,611 clubs carry
this exact byte shape — the rest vary and are not yet mapped
(`OPEN_PROBLEMS.md` §3c; probe: `examples/teamlist.rs`). Not yet surfaced in
the UI.

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

## 6c. Attribute names — 49 of 54, every visible attribute named

**Method: intersect in-game player reports.** An index can carry a name only
if every report showing that name agrees with the decoded value there. One
report pins only the values unique on its screen; five reports of different
profiles — attacking midfielder, winger, deep midfielder, ball-winner and
goalkeeper — leave **all 36 visible outfield attributes uniquely determined**.
`cargo run --release --example namesolve -- <save.fm>` re-runs the solve and
prints the candidate set per index.

The keeper's report is the only way to see the goalkeeping set, which the game
hides for outfielders. One keeper pins four outright (Kicking 15, Throwing 16,
One on Ones 19, Punching Tendency 33 — each unique on his screen). The rest
resolve by combining which indices are goalkeeping ones at all with how each
correlates against Current Ability: FM's three *tendency* attributes say
nothing about how good a keeper is and are exactly the three weakest
correlators — 33 at r=0.245 (ground truth confirms Punching), 31 at r=0.324
and 32 at r=0.459. So 31 is Eccentricity, leaving Command of Area for 13 at
r=0.749, and Rushing Out for 32 by elimination. The prediction made before
any ground truth existed — "the low correlators are the tendencies" — held
exactly.

**24 and 25 are the feet.** Index 25 had been written off as "not an
attribute" because its mean is 17.3 with most players at 20 — which is
precisely what a right-foot rating looks like in a database of right-footed
players. The one left-footer among the five reports reads 20 at index 24 and
9 at index 25, matching his "Very Strong" left foot and "Reasonable" right.

| Index | Name | Index | Name | Index | Name |
| --- | --- | --- | --- | --- | --- |
| 0 | Crossing | 20 | Positioning | 37 | Stamina |
| 1 | Dribbling | 22 | First Touch | 38 | Pace |
| 2 | Finishing | 23 | Technique | 39 | Jumping Reach |
| 3 | Heading | 24 | Left Foot | 40 | Leadership |
| 4 | Long Shots | 25 | Right Foot | 42 | Balance |
| 5 | Marking | 26 | Flair | 43 | Bravery |
| 6 | Off the Ball | 27 | Corners | 45 | Aggression |
| 7 | Passing | 28 | Teamwork | 46 | Agility |
| 8 | Penalty Taking | 29 | Work Rate | 50 | Natural Fitness |
| 9 | Tackling | 30 | Long Throws | 51 | Determination |
| 10 | Vision | 31 | Eccentricity | 52 | Composure |
| 11 | Handling | 32 | Rushing Out Tendency | 53 | Concentration |
| 12 | Aerial Reach | 33 | Punching Tendency | | |
| 13 | Command of Area | 34 | Acceleration | | |
| 14 | Communication | 35 | Free Kick Taking | | |
| 15 | Kicking | 36 | Strength | | |
| 16 | Throwing | | | | |
| 17 | Anticipation | | | | |
| 18 | Decisions | | | | |
| 19 | One on Ones | | | | |

**The last four goalkeeping indices fell to published data.** fminside.net
serves the FM 26.2 database with display×5 values — the same source class
as the FM Scout wage check — so a player page works as a report without a
screenshot. Donnarumma's page splits Handling 80 / Aerial Reach 75 /
Communication 70 / Reflexes 90 where the in-game keeper report read 15 at
all four; his decoded block matches exactly (11=16, 12=15, 14=14, 21=18)
and Alisson's page is consistent with his (17/14/14/17). Chevalier's
distinctive published Punching (display 5) re-confirmed index 33, and the
same pages re-verified the Passing/First Touch, Bravery/Concentration and
Vision splits on four players. Locked by
`published_keeper_attributes_split_the_last_four` in `real_save.rs`.

The five hidden attributes no player screen ever shows fell on 3 August
2026 to the pre-game editor: **41 Dirtiness, 44 Consistency, 47 Important
Matches, 48 Injury Proneness, 49 Versatility**. Haaland's editor sheet
lists them as 9, 16, 14, 10 and 7 — five distinct values matching his
decoded block one-for-one, a single possible assignment — with every
visible attribute on the same sheet matching its named index exactly.
**All 54 indices are named.**

Two labels the statistical era got wrong, both corrected by ground truth:
index 40 was "Aggression" and is Leadership (Aggression is 45), and index 10
was "Passing" and is Vision (Passing is 7). Both were coin flips between two
attributes the first player happened to have equal.

### The superseded statistical method

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

**A third signal was needed for the club that has neither.** Afan Lido in a
2027 career reads eighteen valid player eids in an order transfers destroyed —
4 of the first 6 pairs rising, where the ascending test wants 5 — with *both*
armband slots at `0xFFFFFFFF`, because no captain had been appointed. Neither
signal fired and the whole squad was discarded, so a real Cymru South club
showed no players at all. An unset slot is not a failed one: what identifies a
real record is that each armband is **either unset or one of the club's own
players**, which random bytes do not manage. A list naming the same player
twice is also refused.

The gain scales with how aged the save is, which is what the diagnosis
predicts — a fresh database is still mostly in ascending order:

| save | squads | players linked |
|---|---|---|
| day one | 3,336 → 3,343 | 27,652 → 27,764 |
| a 2027 career | 2,941 → 2,949 | 22,402 → 22,532 |
| a 2030 career | 3,720 → 3,965 | 26,039 → **28,249** |

Precision barely moves: on the 2030 save the share of squad members resolving
to a decoded person goes 99.24% → 99.11%, and squads resolving under half
their members go 25 → 28 — all foreign clubs outside the loaded leagues, whose
players have no person record either way.
`crates/fm-save/examples/squadchain.rs` reproduces the diagnosis: it replays
the head test, reports which heads the ascending run discards, and prints the
anchors, rising-pair count and armband slots for any club eid.

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

### 6d-quater. B and youth squads — the same table's other rows, SOLVED

Found 7 August 2026 chasing a player FM's search showed at Rangers while
Gilet showed no club: the squad table holds **several records per club**, and
the walk above claims only the senior one. Each record sits behind a
separator ending `01 [type] FF [flag]`, then the familiar head:

```
01 [type] FF [flag]           separator; type 0x64 senior, 0x13 B, 0x15 youth
u32   owner_eid               the CLUB's eid for club teams — see below
10x   00
u32   ordinal                 ascends across the whole table, one per record
u32   team_uid                the team entity's own uid
...   variable body
                              (optional FF-marked list as in 6d)
```

Three things stop the obvious parse:

- **The senior row's uid is the club-table uid** — that is what 6d's
  `(eid, uid)` check accepts. The B and youth rows are keyed by the *club's*
  eid but carry their **own team entity's uid** (`0x77xxxxxx` when
  career-generated), so the check can never claim them. What proves them
  real instead is the **ordinal**: it ascends across the entire table, and
  the longest ascending run through it is the table's spine.
- **`0x64` rows include nation teams**, whose owner eids are *nation-team
  entities* (South Korea's 26-man squad sits at entity 79) and collide with
  small club eids. Only `0x13`/`0x15` rows bind, and only above eid 260 —
  the nation range runs to 249. The B and youth squads of the first ~260
  database clubs are the accepted cost.
- **Most rows are empty**, and an unbounded list hunt reads straight through
  an empty record into its neighbour's list — every squad it claims is then
  one club over (the first scan of this table read exactly that way, off by
  one club per empty row). Every separator-anchored head bounds the record
  before it, claimed, senior, nation and empty rows included.

**The 0x64 senior rows bind too — that is where the out-of-league clubs
live** (same day, one step further). A senior row whose uid matches the
club table is 6d's business; the rest are clubs outside the loaded leagues,
their uid regenerated at season rollover. The trap is that **national sides
share the 0x64 type**: men's at nation eids 1–249, women's from 261 up —
squarely inside club-eid space, so Denmark Women at entity 262 would read
as OB's squad. The separator's flag byte splits them: every representative
row in the index save carries **bit 0x20** (`0x24`) where club rows read
`0x00`/`0x04`. The bit is only meaningful on senior rows — a legitimate B
row reads `0x34`. One belt-and-braces veto sits at link time: an
out-of-league list where three or more members already carry a first-team
club whose majority is elsewhere is refused whole — a real out-of-league
squad's members are precisely the people no loaded list knows, and B/youth
lists are exempt because their known members are loanees, who genuinely
point elsewhere.

The lists are where the missing players live: **22,495 players bind outside
the first-team lists on the 2035 index save** (15,788 from B/youth, the
rest senior out-of-league), 5,950 of the senior bindings with *zero*
cross-club conflicts. Conflicted players stay unbound: an ambiguous read is
not a link. Bindings check out against reality: the index case (Jae-Wan
Choi, Rangers B) matches FM's own search; day-one lists put Jay Spearing in
Liverpool's youth setup and Adam Rooney at Barton Town, both true in the
FM26 database; the senior pass reads Khorfakkan, Cruz Azul and the Scottish
B-loan army correctly on the 2035 save. **Day-one saves gain little from
the senior pass** (+245): an out-of-league club's senior row exists but is
*empty* until the game materialises the squad, which is why Willian and
Calleri still show no club on day one — their contracts now decode
(Grêmio and São Paulo deals to 31/12/2026, the Brazilian season's end), but
no list in `game_db` carries them. Secondary lists bind only players no
first-team list claims, and stay out of squad-size and wage-bill sums.

The known-member "majority" of a B list routinely points at *other* clubs —
Hearts' list reads 4 known members, all at Cumnock and the like. Those are
loanees: a B player good enough to be loaned out is bound to the borrowing
club by its first-team list, and the players actually at the club are
exactly the ones no other list knows. `regscan.rs` reproduces the whole
diagnosis; `teambound.rs` prints what the pass binds on any save.

## 6d-bis. Stub people — squad fillers without person records

Squad lists can reference entity ids that no person record answers to. Those
members are **stubs**: ~33-byte entries in their own table,

```text
[01|02] 40 10  00 00 00 00  [eid u32] [uid u32] [uid u32]  07 ...
```

— the same `40 10`-headed identity shape as the in-record objects of §3,
distinguished by the `07` kind byte after the doubled uid. The uid sits in
the generated-player band (~2.0 G). This is how FM stores generated
non-contract signings — the "pay to play" players that fill lower-league
squads. Port Talbot (v03).fm holds 8,849 stubs, 8,846 of them unbound to any
person record; without parsing them, those squad members simply vanish from
a squad list (found 3 August 2026 chasing exactly that report).

`stub::scan_stubs` reads the table; `Save::stubs` keeps the ones a squad
references whose eid no parsed person claims, and the UI shows them as
undecoded members. Fields after the kind byte are **not decoded**: the byte
at +28 is age-shaped (20-22 for known senior fillers) and the u32 at +29
resolves in all three name pools at once — overlapping id spaces make that
ambiguous, so no name is shown. FM displays real generated names for these
players, so the ids exist somewhere; finding the name link is open.

## 6d-ter. Compact people — folded out of the loaded world, SOLVED

An aged save loses people. Kylian Mbappé parses from every day-one save and
is absent from the 2035 one — and his uid (85139014, FM's public database id
for him) occurs exactly **once** in that save's main frame, in a 30-byte
entry that is not a person record:

```text
10 00  [forename id u32] [surname id u32]  01
[00-02] 40 [flags]  04 00 00 00  [eid u32] [uid u32] [uid u32]
```

A name reference, then the same entity-object header and doubled-uid triple
every identity carries (§3; the type and flags bytes vary exactly as they do
there). The entries sit **in the person table in eid order, embedded between
full records** — Ongoing.fm runs …22278 (full, Mandréa), 22279 (compact,
Mbappé), 22280-22282 (compact), 22283 (full, Wissa)… — so this is the person
table's own storage for people who have left the loaded game world (retired,
or playing beyond the simulated leagues), not a reference list. There is no
record prefix, no inline name, no date of birth and no attribute block; the
name ids resolve in the ordinary forename and surname pools (his read
"Kylian" + "Mbappé" — the surname pool's plain entry, where his full record
carried the inline "Kylian Mbappé Lottin").

Population confirms the ageing story: Day One.fm and the Afan Lido save hold
**zero** compact entries, the 2030 Benchmark 180, Port Talbot (v03) 233, the
2035 save 976 — every one with both name ids resolving, and none of them
referenced by any squad list.

`person::scan_compact` reads them with the same acceptance a full record
gets (both name ids must resolve, the uid must be doubled), and `Save::parse`
appends them **after every offset-based pass** — their offsets sit inside
other people's records, and letting them into the identity, contract or
ability passes would shift record boundaries and hand them a neighbour's
data. An eid a full record already claims stays with the record (one or two
per aged save). `Person::compact` marks them; every field beyond name, eid
and uid is `None`, because the save genuinely does not store it.

## 6e. Contracts — wage and expiry SOLVED

The contract lives in the bytes immediately **before** the person's record
prefix, within ~600 bytes. The wage row is anchored on the person's own
entity id:

```
[eid u32] [u32] [00 00 00 00] [wage u32] 01 xx 00 [FF FF FF FF]
```

and the expiry sits earlier in the block as a date pair following a run of
eight `FF` bytes, within 400 bytes of the anchor (measured over 141K
contracts in a 2035 save: all but 13). Other fields visible in the block but
not yet parsed: contract start, the date the deal was signed, and several
smaller money values that look like bonuses and clauses.

**The contract is not the only row anchored on the eid** (found 7 August
2026). Between it and the record prefix other eid-anchored rows can appear —
Jae-Wan Choi (2035 save, Rangers B) carries `[eid][u32][00 00 00 00][u32]
01 00 00 01` eighty bytes before his record, international-duty shaped, with
the true contract 337 bytes back. The hunt therefore walks *backwards
through every occurrence* of the eid until one passes the shape test, rather
than testing only the last; demanding the last occurrence be the contract
had dropped 63,491 of 141,211 contracts in that save — every player whose
contract had such a row behind it showed no wage, and the free-agent filter
(no contract *and* no club) misfiled the ones whose club link is also
missing (§1 residual 1). Some blocks' expiry slot holds day 0 / year 2048
rather than a date — those read as no expiry, and the nearby
`[u16 masked-day][u16 year]` stamps (Choi: day 244 of 2034) are not yet
understood well enough to claim.

Verified against public figures and an in-game report:

| Player | Wage / week | Until | Source |
| --- | --- | --- | --- |
| Haaland | £450,000 | 30/6/2034 | FM Scout, exact on both |
| Salah | £400,000 | 30/6/2027 | real contract |
| Van Dijk | £350,000 | 30/6/2027 | real contract |
| Mbappé | £496,918 | 30/6/2029 | Madrid, EUR converted |
| Musiala (2035 save) | £392,499 | 30/6/2037 | in-game report band £350K-£425K |

Non-round wages are foreign-currency contracts converted into the save's
display currency — the signature of a real read. Haaland's block even holds
17/1/2025, the real-world date he signed his extension. Across the reference
save 20,388 contracts parse; the median weekly wage is £400 (semi-pros), the
90th percentile £18K. People without a matching block — the unemployed and
retired — get `None`, not zero.

Transfer *value* has not been found and is probably computed by the game
rather than stored.

## 6f. In-game shortlists — SOLVED, in `scout_man.dat`

The human manager's shortlists live in the `scout_man.dat` member (§1b), not
in `game_db.dat` — a few KB, located through the manifest and read by
`shortlist.rs`. Decoded 2 August 2026 against a probe save whose shortlists
were created minutes earlier with known contents, then verified end to end:
`ZZPROBE` = van Dijk, Wirtz, Salah, in creation order, resolved through
person entity ids by the ordinary parse.

After the frame's `03 01 "tad."` header, shortlist records follow, one per
list plus lookalike records for scouting focuses:

```
1e 00 06 f8 43 01 00 01        record separator (first record differs)
... flags ...
FF FF FF FF  01 01 00  6c 07   head run; 6c 07 = year 1900
[u32 len] [name]               the list's name; len 0 = the unnamed default
... filter block               "frlp"/"tlif", nation and division ranges;
                               a focus carries "flpn" + "manP" instead
"rSrP" 00 [u32 count]          the player list
per entry, 22 bytes:
  02                           entity type tag
  [u32 person eid]             resolves against Person::eid
  [u32 date added]             the masked day-of-year pair of §1c: day of
                               year in the low nine bits of the first u16
                               (probe entries: 0x1a9f → day 159 = 8 June,
                               the career's current date), then the year
  [u32 0]
  01 00 6c 07                  null date (day 1, year 1900)
  FF FF FF FF
  00
... 75 x u32                   column ids for the shortlist view
```

The parser anchors on `rSrP`, reads the count and entries, and takes the name
from the last head run before the tag. A focus record has no `rSrP` and so
contributes nothing. Entries whose type tag or eid range fails drop the whole
block rather than half-read it.

The tags echo names FM has used for years: `tslf`/`tslm` (shortlist file /
manager — the `.slf` extension) mark the same structures in `humans.dat`,
which holds only filter state, no player lists.

**Writing.** Since the format and the date encoding are both known, edits go
back in: `shortlist::add_entry`/`remove_entry` splice a 22-byte entry and fix
the count, and `archive::replace_member` rebuilds the save around the changed
member — recompressing it, re-tiling every offset after it, rewriting the
manifest, and repointing the header u32 at +9, which sits exactly nine bytes
before the inner `02 01 "fmf." 08 00 00` container that precedes the manifest
frame (verified on all four saves examined). Identity reassembly is
byte-identical on a real save, and an in-memory add survives a full reparse
(`probe_save_survives_reassembly_and_a_shortlist_edit`). New entries write
the day-of-year with the high seven bits zero — an attested value of that
field (§1c observes 0, 13 and 41) — because those bits' meaning is unknown.
**FM accepts the result**: verified 2 August 2026 by loading a rewritten
save in the game and seeing the added player on the shortlist — which also
means FM tolerates a zstd level different from its own and the zeroed high
date bits.

Still unknown: those high seven bits, the flag bytes in the record head, the
filter block's meaning beyond its nation/division range lists, and everything
in the 1.6 MB `shortlist_man.dat` member — descending ranked pools of
`(eid, reputation-like value)` pairs, the AI's candidate lists, with no
strings anywhere.

## 6g. Non-player attributes — SOLVED, on the object one eid below

A person's sheet — the one the pre-game editor calls "All Attributes" — sits
in the **tail of the previous person's record**, behind an identity triple,
exactly the arrangement player attribute blocks use (§6: "blocks sit ahead of
the person they belong to"). Sterling's sheet is behind the triple reading
eid 8401 while he is 8402; Fradley's behind 20129 while he is 20130; Slot's
behind 2057 (Verberne) while he is 2058. The triple in front is nearly always
the *previous person's own identity* — on a day-one save 18,202 of 18,247
sheet-bearing triples match a person's exact (eid, uid) pair — so binding by
`triple eid + 1` covers 18,245 of them.

```text
( entity object header [type 00-02] 40 [flags] [u32] — OR nothing at all )
[eid u32] [uid u32] [uid u32] [tag 01]
  ( optional preamble of 8-byte rows )
  [home rep u16] [current rep u16] [world rep u16] [CA u16] [PA u16]
  [8 bytes filler]
  [54 values, each 1-100]
```

**The header is optional and cannot be required.** Plenty of records write
the identity bare — three zero bytes, then the triple — and requiring the
header cost 7,447 sheets on a day-one save (10,800 found against 18,247).
Arne Slot's CA-165 sheet was one of them, behind Verberne's headerless
identity, which is what "staff profiles show nothing" reports trace to. The
same headerless shape also defeats the shadow-hit test that relies on the
header (`person.rs`): the one-byte-early read binds `eid << 8` / `uid << 8`
(Verberne read as 526592 / 153885696), so the scanner now also drops a hit
whose ids both end in a zero byte when the next offset reads them shifted
back.

* **Reputations are the database's own numbers**, scaled to 10000 as the
  editor shows them — divide by 50 for the 0-200 value. Order is
  **home, current, world**, which is not the order the editor prints. Fradley
  reads 6350 / 7000 / 5500 against an editor page saying 7000 Current, 6350
  Home, 5500 World; Nikolić 6250 / 6500 / 4500 against 6500 / 6250 / 4500.
  Career start does not rewrite them. The earlier conclusion that it did came
  from reading the neighbouring person's object.
* **CA and PA follow, 0-200**, and are the same bytes `ability.rs` reads at 39
  and 37 back from a player's attribute block.
* **The fields sit at no fixed stride from the tag byte.** Some objects carry a
  preamble of 8-byte rows first, so they must be found by signature — both
  abilities sane and the 54 bytes eight past them all 1-100 — not by offset.
* **The block is the editor's flat 52-item list at slots 2-53.** `s0` is a
  small enum (14 values seen) and `s1` reads 12 in 97.7% of blocks: a two-byte
  header, which is exactly the 54 − 52 the item count demands.

| slots | items | storage |
|---|---|---|
| 2-27 | 1-26, Attacking through Width — the tendency half | raw 1-20 **on a day-one save** |
| 28-53 | 27-52, Coaching through Coaching Set Pieces | raw × 5 |

with a per-person drift of a point or two on the second half, which rounding to
nearest removes. It is the editor's **raw** column, not its controlled one, and
the scale is five rather than the four an earlier single-person fit suggested.

**The tendency half does not survive ageing.** In a 2030 career Klopp's low
slots read up to 98 and Guardiola's up to 88 where the editor caps at 20 — an
aged save rewrites the tendency half onto an internal scale that is not
decoded (compare player attributes, whose 1-100 internals only *initialise*
to display×5). One value past 20 proves the whole half is off the editor
scale for that person; the UI shows no number there rather than one on an
unknown scale, and staff scoring skips those slots. The ÷5 coaching half
still reads 1-20 on aged saves and stays plausible (a generated physio reads
Physiotherapy 17, coaching 1-2), but has no aged ground truth yet.

Two of the 52 rows the pre-game editor leaves unnamed itself; `staff.rs` keeps
them `None` rather than guessing.

Where a person's database row is blank — which is most staff — the game
generates the sheet at career start, so those values are the save's own and
match no editor page. Gerig's thirty-nine editor values are all 0 and his block
is dense.

Verified in `real_save::staff_sheets_match_the_editor`: 34 values across
Nikolić and Fradley, reputations included, every one exact — plus Slot's
CA/PA and reputations from behind the headerless identity.

## 6h. The roster table — one entry per club, manager slot SOLVED

A dedicated table (~0x20d0000 region in the reference save, far from the
club records) holds one entry per club in entry-entity order:

```text
[eid2 u32] [club uid u32] [club uid u32]  0a 00  [3 bytes] [u32] [u32]
[FF FF FF FF] [u32] [manager eid u32 | FF FF FF FF]  ...
[count u16] [player eid u32] × count  ...
```

`eid2` is the entry's own entity id (club eid + 131 on day one — a gap, not
a law). The doubled uid is the club's, the same validation squad records
offer. **The slot two u32s past the `FF` run is the club's manager**:
Slot / Arteta / Guardiola exact on a day-one save, Iraola at Liverpool in a
2030 career, `FF FF FF FF` when the seat is empty — 1,646 filled slots on
day one against 17,207 vacant, and the four out-of-band values are noise
the scanner rejects. `backroom::scan_managers` reads it; `Save::parse`
binds each manager's `club_eid`.

What the rest of the entry is **not**: the u32 before the manager resolves
to implausible people as a person eid (left undecoded); big clubs carry one
extra word after the manager (637 on all 29 on day one) before the shared
`00 FF FF FF FF` tail; and the count-prefixed list further in is the club's
**player registration list** — Liverpool's reads Mac Allister, Szoboszlai,
Alisson — which the squad table already covers.

**The rest of the backroom sits in the club record's own body** as several
count-prefixed person-eid lists — FM's staff categories. Liverpool's
day-one record holds runs of 20, 35 and 39, Hulshoff in the 35, the
manager in none (he lives in the roster slot above); every member of every
run checked was pure staff. Day-one lists are ascending and sit within
6KB of the head; an aged career **shuffles them and pushes them deeper**
as the record grows — a 2031 career's Port Talbot lists sit 18KB in,
still inside the club's own span, verified name-by-name against the
running career's staff screen (Truck, Nicholas, Evans). Order is
therefore not part of the shape; acceptance is the count byte, in-range
ids, and the caller's gate — four in five members must resolve to
non-player people, which random bytes or a run of another entity kind
cannot pass. `backroom::scan_staff_lists` reads them, `Save::parse`
binds: 6,653 employed staff on a day-one save, 6,641 on the 2031 career.
(A note for anyone re-deriving this: comparing offsets from two dumps of
a save FM is actively rewriting produces phantom "wrong span" results —
snapshot the file first.)

**The lists come as a department triple.** A club's three lists are
written back to back with no separator, in a fixed order: **medical,
coaching, recruitment** — verified member-by-member against a running
career's staff screen (the medical run held exactly the physios, doctors
and sports scientists; coaching the coaches, assistant managers, fitness
and GK coaches, analysts and the head of youth development; recruitment
the scouts, chief scout and technical director). A second adjacent triple
is the club's B or women's team. `backroom::Department` labels exactly-
three adjacent runs; anything else keeps the club link with no
department, and the roster seat marks the manager — together they give
each employed staff member a `Person::staff_role`. The director of
football and chairperson sit in none of the lists and stay unbound.

Rows shaped `01 [tag u8] [uid u32][uid u32]` nearby resolve to no person
(team-entity references, by the look of the ids); not person roles.

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
