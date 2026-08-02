# Open problems

Handoff notes for the parts of the Football Manager 26 save format that are not
solved. Each entry says what the problem is, what was tried, what the evidence
actually showed, and where I would go next.

Everything here was worked against `Career.fm` (FM 26.0.0, in-game date
26 October 2025, 12,396 people, 3,999 of them players). What *is* solved lives
in `SAVE_FORMAT.md`; this file is only the remainder.

The Python spikes referenced below are in `research/` and in the session
scratchpad. They all operate on `frames/f0003.bin`, the 105 MB main database
frame, which `research/unpack.py` produces.

---

## 1. Squad membership — SOLVED, residuals remain

Solved on 1 August 2026; the format is documented in `SAVE_FORMAT.md` §2, §3
and §6d and shipped in `strings.rs`, `person.rs`, `club.rs` and `squad.rs`.
The two claims that had made it look unreachable were both wrong:

- **"Person records carry no unique identifier"** — they do. Every person
  record contains an identity block `[eid][uid][uid]` (three zero bytes
  before it), and eids ascend strictly through the person table. The earlier
  sweeps missed it because they looked for *club* references near players,
  not for the player's own id being referenced *from elsewhere*.
- **"The parser sees all the people"** — it saw a third of them. Inline full
  names exist only when they differ from forename + surname; the other
  ~37,000 people (van Dijk, Rice, Isak...) have a zero-length name field and
  resolve through the sectioned string table.

The eight u32s after the club short name were a red herring — they resolve to
unrelated small clubs (Badalona, Pittsburgh Riverhounds), not teams. The real
link is a dedicated squad table validated against club `(eid, uid)` pairs.

**The club flags byte, fixed 2 August 2026.** The byte before the club
record's `FF FF` tail is per-club flags, not the constant 0x10 the scan
assumed (`SAVE_FORMAT.md` §4). Tottenham and Chelsea read 0x12 in the Afan
Lido save, so both clubs — and with them their whole squads — were silently
dropped, and their contracted first-teamers (Sarr, Bissouma) showed no club.
The scan now accepts any flags byte when the entity head validates; the fix
recovered ~1,050 clubs and 77 squads in that save. Diagnose this class of
problem with `research/clubsig.py` and the `diagnose` example.

**Residuals, in order of value:**

1. **Players at clubs outside the loaded leagues have no squad list at all.**
   In the Afan Lido save 6,585 of 27,483 contracted players resolve to no
   club, and probing (diagnose example, "where do the orphans' squad lists
   hide") shows their eids and uids appear in *no* FF-marked count list in
   `game_db` — Brazilian, Argentine and MLS squads among them (Willian,
   Calleri, Acosta). The squad table only materialises for active divisions;
   FM must derive the rest from something else, likely the contract block,
   whose club link — if it exists — is not yet found. A naive scan of the
   ±384 bytes around the person record for known club eids matches only
   noise. Next: diff two same-club orphans' full records against a
   third-club orphan for a shared u32.
2. **38 of 15,558 squad-referenced eids do not resolve** (0.24%). They are
   scattered among low eids (1007–1363) plus eid 1 (Maldini — his identity
   block `[1][45][45]` loses the LIS race because his uid, 45, is unusually
   small and something noisy precedes him). Affected people show no club.
   Diagnose with `research/pipeline_v2.py`.
3. **Common names are not used for display.** People with a `common_name_id`
   ("Juanito") still display forename + surname or the inline name; the
   common-name pool is parsed but unused.
4. **The women's-club ambiguity.** Two club entities can share a short name
   (both Manchester Citys). The UI labels players with the short name, so a
   club *filter* built on names conflates them; filter on club eid instead.
5. **Junk contract rows read as contracts.** A tail of records parse as
   wage 0 expiring 2 January 1900 (the null date + 1) — free-agent or
   variant layouts that should read as no contract, but currently show one.

(The earlier "squad table stops at club eid 15986" residual is closed: with
the flags byte fixed, squads parse up to the highest club eid in the table —
20,677 in the Afan Lido save.)

---

## 2. Attribute names — 49 of 54, every visible attribute named

Solved by intersecting five in-game player reports (see `SAVE_FORMAT.md`
§6c), then finished on 2 August 2026 with **published FM 26 databases as a
second ground-truth source**: fminside.net serves the 26.2 DB with display×5
values — the same source class as the FM Scout wage check — and its player
pages act as extra "reports" without a screenshot.

**The keeper four are solved.** Donnarumma's page splits Handling 80 /
Aerial Reach 75 / Communication 70 / Reflexes 90 where the one in-game
keeper report read 15 across the board; his decoded block matches exactly
(indices 11=16, 12=15, 14=14, 21=18), Alisson's is consistent, and
Chevalier's distinctive published Punching (37 internal → display 5, decoded
5 at index 33) re-confirmed the tendency trio. The same pages re-verified
the Passing/First Touch, Bravery/Concentration and Vision assignments on
four players each. Locked by `published_keeper_attributes_split_the_last_four`
in `real_save.rs`.

**Remaining: indices 41, 44, 47, 48, 49** — the five hidden attributes
(Consistency, Important Matches, Injury Proneness, Dirtiness, Versatility in
some order). No player screen shows them. efem.club publishes hidden values,
but its numbers failed the consistency test that fminside's passed: no
assignment of the five names fits all five probe players (Chevalier's
published Versatility 14 collides with every candidate index), so nothing
was shipped from it. Partial evidence worth keeping: index 41 is low for
everyone and closest to **Injury Proneness** (inverse of efem's "Injury
Resistance"), and 47 tracks efem's **Important Matches** on four of five
players. The clean finish is one in-game editor screen of any player's
hidden attributes against `cargo run --release --example player`.

### The earlier state of this problem

Solved substantially on 1 August 2026 via ground truth: an in-game player
report (Jamal Musiala, aged save) checked against his decoded block. All
fourteen statistical labels verified exactly — **the Marking/Tackling caveat
is closed** and **Pace (38) against Acceleration (34) is split** — plus six
new labels (Flair 26, Corners 27, Work Rate 29, Free Kick Taking 35) and one
correction: index 40 is **Leadership**, not Aggression. Details and the
per-index table are in `SAVE_FORMAT.md` §6c.

Same session found that attribute internals are 1-100, only *initialised* to
display×5 — an aged save has no multiples of 5 left, which had made it parse
as zero players until the scan learned the structural signature.

**Remaining, and the cheap way to finish it.** Several indices tie because
Musiala shows the same displayed value on each: {7, 22} = First Touch /
Vision, {43, 53} = Bravery / Concentration, {24, 45} = Aggression + a hidden
attribute, and the 18/16/14 pools hold Dribbling, Composure, Agility,
Anticipation, Decisions, Determination, Balance, Long Shots, Teamwork,
Natural Fitness, Stamina among hidden attributes. **Each further screenshot
of a player with a different value spread breaks more ties** — ideal is a
limited player whose visible attributes are all different numbers. The
`player` example prints any player's full decoded block for comparison:

```bash
cargo run --release --example player -- <save.fm> "Player Name"
```

**Method notes that still matter.** Player-topping evidence alone is never
sufficient (index 6 looked like Heading until positions showed centre-backs
at 8.5 there); and a screenshot only pins an index when its value appears
exactly once on that screen.

---

## 3. Goalkeeping attribute names — SOLVED

All eleven goalkeeping indices are named: 11 Handling, 12 Aerial Reach,
13 Command of Area, 14 Communication, 15 Kicking, 16 Throwing, 19 One on
Ones, 21 Reflexes, 31 Eccentricity, 32 Rushing Out Tendency, 33 Punching
Tendency. The last four fell on 2 August 2026 to published-database ground
truth (§2): Donnarumma's fminside page splits them where the one in-game
keeper report read four fifteens, and the CA-correlation ordering that had
grouped core skills against tendencies is consistent with the final naming
(Handling and Reflexes are the top correlators; Eccentricity and Punching
the bottom).

---

## 3b. Staff attributes — structure found, ids unmapped

Superseded in part: the person record holds a **second entity object** (see
`SAVE_FORMAT.md` §3), and for player-people it carries a full second 54-byte
attribute block — their non-player attributes. For pure staff the coaching
data is an id→value row list instead; the values are 1-100 and Emery's
nation-knowledge rows read exactly 100 where his screen says "Complete".
What remains is mapping row *ids* to attribute names, which one in-game
editor screen of a staff member would settle.

The 2 August 2026 session opened Emery's record up (`staffrows` example,
Career.fm, record at `0x413b724`):

1. **His identity block exists but loses the LIS race.** At +436 the record
   reads `[eid 135][uid 5193][uid 5193]` — and 5193 is exactly the id
   efem.club uses in its staff URL, confirming uids are FM's public DB
   ids. Staff eids are low and out of order among the surrounding records,
   so `bind_identities` drops them and `Person::eid` reads `None`. The fix
   is a second pass that accepts leftover blocks by shape and uid
   uniqueness rather than chain order; that would anchor every pure-staff
   record.
2. **The row list is knowledge, and it parses.** From ~+88: 16-byte rows
   `[u32 id] [00 x4] [u16] [03 01|01 03] [value u8] [4F|4E|00] 00 FF`.
   Emery's four value-100 rows are the "Complete" nation knowledge already
   predicted; the ids are entity ids (nations/cities) and the other rows
   hold 40-90 values — knowledge levels.
3. **A 1-100 run at ~+458 is the likely coaching block** (~24 values,
   62/67/57/62/10/67/... — two 67s matching Emery's published Set Pieces
   and GK Shot Stopping, a 10 matching GK Handling). It does not contain
   his published 92/100 values, so either the scale or the slot map is
   still wrong — unresolved.

**Published staff ground truth exists** for the mapping: efem.club serves
Emery's full FM 26 coaching sheet in both 26.0.0 and 26.2.0 versions
(26.0.0: Attacking 80, Defending 78, Fitness 80, Mental Coaching 36,
Tactical 92, Technical 82, Set Pieces 67, Working With Youngsters 81, GK
Shot Stopping 67 / Handling 10 / Distribution 12, Adaptability 22,
Determination 82, Level Of Discipline 100, Motivating 80, People
Management 79, Judging Staff Ability 84, Negotiating 82, Tactical
Knowledge 81, Analysing Data 13, Judging Player Ability 82, Judging Player
Potential 82, Physiotherapy 33, Sports Science 27) — a slot → name key
waiting for the block above to be aligned.

**The attribution problem is solved; the field turned out not to be
reputation.** Worked 2 August 2026; `repcheck`, `secondobj` and `objmap`
examples reproduce everything below.

What the structure actually is:

1. **The `02 [A][B][C]...` run sits directly after the person's *own*
   identity block, inside their record** — Saka at +222 from the record
   prefix, van Dijk +289, Musiala +594. No `[eid+1]` search is needed at
   all; the earlier contamination worry was aimed at the wrong landmark.
   On aged saves the run can be preceded by one or two 30-byte
   `10 00`-tagged blocks (`10 00 [u32][u32] [flags] 04 00 00 00
   [eid][uid][uid]`) — entity *references* with their own repeated-uid
   pairs; skip them and the `02` run follows.
2. **Eids are per-save indexes; uids are persistent.** Haaland's second
   object reads eid 10242 / uid 29179299 in Career.fm and eid 12400 /
   uid 29179299 in the 2035 save — same uid, renumbered eid. His player
   uid 29179241 is also exactly the id sortitoutsi and fmscout use in
   their URLs, so uids are FM's public DB ids.
3. **Most records hold only the person object.** An `objmap` sweep finds a
   second in-record `[eid][uid][uid]` block in ~1,100 of 49,217 records
   (67 with consecutive eids, Haaland among them). The doc claim that every
   person carries two objects is wrong as stated; the non-player object
   usually lives elsewhere and is *referenced* by the `10 00` blocks.
4. **The run is not reputation.** With attribution record-bounded, the old
   readings reproduce exactly (Chevalier 5892, Musiala 2690 in the 2035
   save) — and published ground truth (efem.club, FM 26 DB) orders
   reputation Haaland 96 > van Dijk 93 > Saka 90 > Musiala 87 > Chevalier
   70, while the run reads Chevalier 6400 > Haaland 5250 in the same save.
   Uncorrelated; whatever A/B/C measure, it is not the editor's reputation
   triple. The `D == B/50` invariant that made it look derived holds only
   on fresh saves — 14,582 of 27,368 in Career.fm, 1,356 of 37,113 in the
   2035 save — an initialisation artifact, like attributes starting at
   display×5.

**Where to go next:** the run's semantics are open (values move between
saves — Saka 5550 → 5800 over a good season — so it is live state, not DB
identity; candidates worth testing against in-game screens: happiness,
morale-adjacent state, fan/media standing). Actual reputation is not yet
located anywhere in the save; a fresh sweep should search for the known
0-100 published values ×100-ish encodings near the person and second
objects.

### The earlier dead ends

Staff show 19 attributes in-game: nine coaching (Attacking, Defending,
Fitness, Goalkeeping, Possession, Set Pieces, Tactical, Technical, Working
With Youngsters), five mental (Adaptability, Authority, Determination,
Motivating, People Management) and five knowledge (Judging Player Ability,
Judging Player Potential, Judging Staff Ability, Negotiating, Tactical
Knowledge). None are parsed; the player/staff split is the *absence* of the
54-byte block, so staff carry no numbers at all.

**What was tried, with two staff reports as evidence** — Unai Emery
(Arsenal manager: Tactical uniquely Outstanding, Technical and Working With
Youngsters Very Good, Goalkeeping Average, Set Pieces Competent, the rest
Good) and an unemployed manager whose whole row is Unsuited bar three:

1. **Shape search in report order.** Sweep every 9-byte window within
   ±20,000 bytes of Emery's record for one ordered like his coaching row.
   **Zero windows match** on either the 1-20 or 1-100 scale. The likely
   reason is that storage order is not screen order — the same is true of
   player attributes, where the report groups by Technical/Mental/Physical
   while the block does not.
2. **Differential against a weak manager.** Look for offsets, relative to
   each record's prefix, where the elite manager reads 10-20 and the
   unemployed one reads 1-4, which any attribute region would satisfy.
   **No run of three or more.** Staff records are variable-length, so fixed
   relative offsets do not line up between two people.

**Why generalists cannot crack it, and specialists can.** FM shows staff
attributes as words, so a manager's report gives only a rank ordering — and
an ordering is worthless while storage order is unknown, since some
permutation of the block fits any ordering. A *specialist* is different: a
goalkeeping coach reads high on Goalkeeping and at the floor on everything
else, so whichever index spikes for him and sits low for other staff **is**
Goalkeeping. One specialist pins one index regardless of storage order, and
a handful of them pin the block.

The specialists worth collecting, each pinning the attribute in brackets:
goalkeeping coach (Goalkeeping), fitness coach (Fitness), head of youth
development (Working With Youngsters), physio (Physiotherapy), chief scout
(Judging Player Ability and Potential), director of football (Negotiating).
A first-team coach with a lopsided attacking or defending profile pins those
two. Each needs the full attribute list visible and the person's name, so
their record can be found.

A smaller lead worth keeping: the eight-byte 1-20 run just past the date of
birth (`12 13 0f 0e 10 10 12 04` shapes) appears on players *and* staff, so
it is more likely the hidden personality set (Ambition, Loyalty, Pressure,
Professionalism, Sportsmanship, Temperament, Controversy plus one more) than
anything coaching-specific.

---

## 4. Contracts — wage and expiry solved, the rest of the block open

Wage and contract expiry parse (see `SAVE_FORMAT.md` §6e): the block sits
just before the person's record prefix, wage anchored on the person's own
entity id, expiry after an 8×FF run. Verified exactly against FM Scout's
Haaland (£450K to 30/6/2034) and Musiala's in-game report in the 2035 save
(£392,499 inside the scouted band, to 30/6/2037).

**Transfer value is not stored.** The asking-price range on a player's header
is computed: Bouaddi's screen shows £73M-£219M and neither 73,000,000 nor
219,000,000 appears *anywhere* in the 154 MB frame, in any of u32, thousands
or float32. The 1:3 ratio of every observed range (£23M-£70M, £73M-£219M)
says the game derives both ends from one internal figure it recomputes from
ability, age, contract and reputation. Replicating that formula would mean
inventing a number, so the tool shows wage and contract instead.

Still in the block, unparsed: contract start date, signing date (Haaland's
holds 17/1/2025 — the real-world date of his extension), and several smaller
money fields that look like appearance/goal bonuses and clauses (£85,000 and
£82,000 shapes in Haaland's). A player's full Contract tab screenshot against
`cargo run --release --example player` output would name them the same way
the attribute screen named the attributes.

Roughly a third of eid-anchored candidates fail the strict structure test and
keep `None` — variant layouts (part-time, youth, non-contract) not yet
mapped. Transfer *value* is probably computed by the game, not stored.

---

## 5. Nation names — 150 named, five doubtful groups remain

Every occurrence of "England" in the database frame is the *surname* England;
the country names live in FM's localisation files, not the save. They are
named anyway by grouping people by nation identifier and reading the best
players in each group, which is unmistakable — Courtois, De Bruyne and Lukaku
fix 131; Kvaratskhelia and Mamardashvili fix 144.

**150 identifiers are named.** The 1 August 2026 pass took the tail down to
groups of three: 73 more nations identified from their best players, each
cross-checked two ways — the id bands are regionally alphabetical (Africa
0–50, Asia 51–91, CONCACAF 92–125, UEFA 126–176, with later admissions
appended at 200+), and every player-based identification landed in its
alphabetical slot (Cyprus 136 between Croatia 135 and Czech Republic 137,
Estonia 140 / Faroe Islands 141 between England 139 and Finland 142, Peru 194
between Paraguay 193 and Uruguay 195). All 73 were then re-checked against a
second, aged save whose regen names carry the right nationality flavour
(Azeri diacritics under 130, Sinhalese names under 81, the Buffonge family
under Montserrat 207).

Five groups stay numeric, deliberately: 204 (Tigrinya names — Eritrea and
Ethiopia cannot be told apart), 93 (Dutch-Caribbean, probably Aruba on the
alphabetical slot but only one weak player), 205 and 206 (minor British
overseas territories, no recognisable player), and 1535 (three players,
suspicious out-of-band id). `cargo run --release --example nations --
<save.fm>` prints the unnamed groups with their best players; anything
identifiable can be added to `nation_name`. A wrong flag is worse than a
number, so the doubtful ones stay as raw identifiers.

### Original note

Twenty nations are named by grouping people by nation identifier and reading the
surnames, which makes a national squad unmistakable (143 gives Zidane, Henry and
Deschamps). But the country names themselves are not in the database frame:
every occurrence of "England" turns out to be the *surname* England, sitting in
the surname table between Boateng and Mullen.

The names are presumably in FM's localisation files rather than the save. If a
full mapping is wanted, that is where to look — or continue the surname-grouping
method, which is slow but works and needs no new format knowledge.

---

## 6. In-game date on FM 26.2.0 saves — SOLVED, one residual

Solved on 2 August 2026; the encoding is documented in `SAVE_FORMAT.md` §1c
and shipped in `gamedate.rs`. The main frame's week stamp at `game_db` offset
0x2A packs the day of year into the **low nine bits** of its u16 — the earlier
"4821 is not a day of year" observation was the packing, not a dead end
(4821 & 0x1FF = 213). Masked, the 2035 save reads within four days of its
known true date, and the Afan Lido save reads 8 June 2026, exactly matching
the current-date stamps repeated through its competition frames. The masked
read is gated on the header's format-version string because 26.0.0 keeps a
different quantity at the same offset that masks to a plausible wrong date.

The header-frame value at offset 50 on 26.2.0 turned out to be
career-constant (same bytes across saves of one career, low nine bits = the
real-world day the career was created) — a creation stamp, not the current
date, which is why the whole-frame scan never found a valid pair.

**Residuals:**

1. **The stamp's high seven bits are unread** (0, 13, 41 observed). Decoding
   them might remove the up-to-a-week staleness; nothing depends on it.
2. **The exact date exists in `rgman/comp_*.dat` members** — several
   competition frames repeat the true current date — but which competitions
   carry it varies by career, so no rule was found worth trusting. Diagnose
   with `research/datehunt.py`.

---

## 7. Index 25 is not an attribute

Within the 54-byte attribute block, index 25 behaves unlike the rest: mean 17.30
with most players at or near 20, against a typical attribute mean of 9–12. No
1–20 attribute distributes that way. It may be a flag or a scaling factor that
happens to sit inside the block. Currently displayed as an attribute, which is
probably wrong.

---

## 8. Writing a shortlist FM can import — CLOSED BY POLICY, not unsolved

Worked on 1 August 2026 against two shortlists exported from FM 26. The
container, the archive manifest and the per-member block framing are **solved**
and written up in `SHORTLIST_FORMAT.md`. A shortlist is an `afe.` archive whose
members are a `.slf`, a thumbnail and a `_data/details.aom`; the manifest is a
single zstd frame in an inner `fmf.` container at the tail.

The member payloads are **encrypted** — `encrypt(zstd(plaintext))` with a
random per-file nonce and a constant 40 bytes of crypto framing, proven by two
files whose identical 10-byte `.img` plaintext shares no byte of its 63-byte
block. FM 2023 shortlists are the same, so there is no plainer legacy format
to target.

Do not pick this up looking for a decoding trick. There isn't one: it needs
SI's key, and taking that from `GameAssembly.dylib` is ruled out by
`LEGAL_NOTES.md:73` and §8.4, which make encryption the explicit stop condition
for this project. The route back in is asking SI for the format, not the binary.
If that ever happens, only the cipher is missing — everything around it is done.

**The "Genie Scout writes these" claim did not survive** — see the correction
in `SHORTLIST_FORMAT.md` §4. The version 07 sample once attributed to Genie is
far more likely an FM-written file from 2019, and the community position is
that Genie still writes bare `.slf` files FM cannot import. Do not cite Genie
as evidence about the key either way; the encryption conclusion stands on the
FM 26 samples alone (random per-file nonce, 16-byte tag, predicted zstd magic
absent).

---

## 9. The human's shortlists inside the save — SOLVED via a probe save

Worked 2 August 2026, looking for a route to shortlist import that does not
touch the encrypted `.fmf` (§8): the save itself is unencrypted, so if the
in-game shortlist is stored there, importing it is just more save parsing.
It is, and it now is — `SAVE_FORMAT.md` §6f has the format, `shortlist.rs`
reads it, and the app lists a loaded save's shortlists for one-click import.

**Found on the way: the save member manifest** (`SAVE_FORMAT.md` §1b). The
last frame names every other frame; `manifest.rs` parses it and
`research/members.py` extracts any member in one seek. That is what turned
"a 106 MB haystack" into "read the 6 KB `scout_man.dat`".

**How it was cracked.** Every save on the machine predated its shortlists —
the "Wirtz"/"WirtzNew" `.fmf` exports of 1 August were created *after* the
newest save was written, and `WirtzNew` appeared in no frame of any save
(checked exhaustively, all 1,375 frames of Ongoing.fm included). So a probe
save was made in FM with known contents — `ZZPROBE` = van Dijk, Wirtz,
Salah — and its `scout_man.dat` gave up the record format in one hexdump:
names in the clear, members as `(02, eid)` entries. The real-save test
`probe_save_reads_the_in_game_shortlists` locks it in.

**Dead ends worth not repeating:**

- **`shortlist_man.dat`** (1.6 MB) is the *AI* side: ~57 sections of
  `(u32 eid, u32 reputation-like value)` pairs sorted descending — candidate
  pools, no strings, no human lists. Undecoded past that.
- **`humans.dat`** holds `tslm`/`tslf` records whose bodies are filter state
  only (`tlif` blocks, `rftn` nation ranges, `rfvd` division ranges) — the
  right tags, the wrong member. The player lists live in `scout_man.dat`.

**The date-added field fell to the date work, not more probing.** The low
u16 that looked "too big for a day-of-year" (`0x1a9f`) is the §1c masked
pair: day of year in the low nine bits (159 = 8 June, the probe career's
exact current date), unknown high seven bits (13 here; 0 and 41 elsewhere).
Two sessions decoded the two halves independently and they locked together.

**Writing works too, verified in FM itself** — decision recorded in
`LEGAL_NOTES.md` (2 August 2026): `archive.rs` rebuilds a save around a
changed member (identity reassembly byte-identical on the real probe save),
`shortlist.rs` splices entries, and the app's detail panel adds/removes
players on in-save shortlists, backing the save up to `.gilet.bak` first.
The acceptance test passed the same day: FM loaded a rewritten
`Probe Edited.fm` cleanly and showed Haaland on ZZPROBE — recompressed
member, re-tiled offsets, rewritten manifest, zeroed high date bits and all.

**Still open from this work:** the high seven bits of the masked day pair,
and everything in `shortlist_man.dat` past the pool shape.

---

## Reproducing any of this

```bash
cargo run --release --example dump    -- <save.fm>   # parsed output
cargo run --release --example diff    -- <a.fm> <b.fm>
cargo run --release --example findattrs -- <save.fm>
cargo test --workspace                                # 54 tests, 9 against a real save
```

The integration tests in `src-tauri/tests/journeys.rs` assert the decoded
positions, nationalities and named attributes against real players, so anything
that breaks an offset fails loudly rather than quietly mislabelling players.
