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

**Residuals, in order of value:**

1. **38 of 15,558 squad-referenced eids do not resolve** (0.24%). They are
   scattered among low eids (1007–1363) plus eid 1 (Maldini — his identity
   block `[1][45][45]` loses the LIS race because his uid, 45, is unusually
   small and something noisy precedes him). Affected people show no club.
   Diagnose with `research/pipeline_v2.py`.
2. **The squad table stops at club eid 15986** in the walk — clubs above that
   (≈2,000 of 17,495 with heads) were not checked for squad records. Most are
   tiny clubs with no employed people, but it has not been proven.
3. **Common names are not used for display.** People with a `common_name_id`
   ("Juanito") still display forename + surname or the inline name; the
   common-name pool is parsed but unused.
4. **The women's-club ambiguity.** Two club entities can share a short name
   (both Manchester Citys). The UI labels players with the short name, so a
   club *filter* built on names conflates them; filter on club eid instead.

---

## 2. Attribute names — 45 of 54, only hidden ones and four keeper skills left

Solved by intersecting five in-game player reports (see `SAVE_FORMAT.md`
§6c). **All 36 visible outfield attributes are named**, plus seven of the
eleven goalkeeping ones and both feet.

Remaining, in order of how cheaply they fall:

1. **Indices 11, 12, 14, 21** — Aerial Reach, Communication, Handling,
   Reflexes in some order. The one keeper seen reads 15 at all four. **One
   more goalkeeper report where those four differ finishes it**; run
   `cargo run --release --example namesolve -- <save.fm>` after adding the
   report to `namesolve.rs`.
2. **Indices 41, 44, 47, 48, 49** — hidden attributes (Consistency,
   Important Matches, Injury Proneness, Dirtiness and similar). No player
   screen shows them, so this needs the in-game editor rather than a report.

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

## 3. Goalkeeping attribute names — group known, individuals not

All 11 goalkeeping indices are identified (11, 12, 13, 14, 15, 16, 19, 21, 31,
32, 33) but not named individually. FM's set is Aerial Reach, Command of Area,
Communication, Eccentricity, Handling, Kicking, One on Ones, Punching Tendency,
Reflexes, Rushing Out Tendency, Throwing.

**A promising split, not yet acted on.** Correlating each against Current
Ability among the 335 goalkeepers separates them cleanly into two groups:

| Index | r vs CA | Reading |
| --- | --- | --- |
| 11 | +0.811 | core shot-stopping skill |
| 21 | +0.802 | core |
| 19 | +0.800 | core |
| 13 | +0.749 | core |
| 14 | +0.720 | core |
| 16 | +0.642 | core |
| 15 | +0.637 | core |
| 12 | +0.541 | core |
| 32 | +0.459 | weaker |
| 31 | +0.324 | **tendency** |
| 33 | +0.245 | **tendency** |

The low correlators are almost certainly FM's *tendency* attributes —
Eccentricity, Tendency to Punch, Tendency to Rush Out — because a keeper being
eccentric or punch-happy says nothing about how good they are, whereas Reflexes
and Handling are most of what Current Ability measures. Index 31 also has the
lowest mean (8.13) and reaches 1, which fits Eccentricity.

That is enough to label the three tendencies as a group, but not to tell them
apart. **The Musiala method finishes this too**: one in-game report of a
goalkeeper, compared against their decoded block with
`cargo run --release --example player`, names every index whose displayed
value appears once on the screen.

---

## 3b. Staff attributes — structure found, ids unmapped

Superseded in part: the person record holds a **second entity object** (see
`SAVE_FORMAT.md` §3), and for player-people it carries a full second 54-byte
attribute block — their non-player attributes. For pure staff the coaching
data is an id→value row list instead; the values are 1-100 and Emery's
nation-knowledge rows read exactly 100 where his screen says "Complete".
What remains is mapping row *ids* to attribute names, which one in-game
editor screen of a staff member would settle.

The reputation-candidate triple after the second identity block (`02 [u16
×3] [u16 pair]`) stays unresolved: Chevalier and Soulé (both Worldwide) read
5892/6257 while Musiala (also Worldwide) reads 2690, so the badge is not a
simple threshold on any of the five values. Do not ship it as reputation.

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

## 5. Nation names — 75 named, the long tail remains

Every occurrence of "England" in the database frame is the *surname* England;
the country names live in FM's localisation files, not the save. They are
named anyway by grouping people by nation identifier and reading the best
players in each group, which is unmistakable — Courtois, De Bruyne and Lukaku
fix 131; Kvaratskhelia and Mamardashvili fix 144.

**75 identifiers are named**, covering over 90% of players (asserted in
`journeys.rs`). What is left is the tail: groups of fewer than twenty people,
mostly small Caribbean and Pacific nations, where the best players are not
recognisable enough to be sure. `cargo run --release --example nations --
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

## 6. In-game date on FM 26.2.0 saves

The date is a `(u16 day_of_year, u16 year)` pair in the header frame, and on
FM 26.0.0 exactly one such pair exists, at offset 50. On 26.2.0 saves the same
offset holds `d5 12 ea 07` — year 2026 reads correctly but the first `u16` is
4821, which is not a day of year. A whole-frame scan finds no valid pair at all.

`gamedate.rs` returns `None` rather than guessing, and the UI says "date
unknown" and falls back to the system clock. Worth fixing because ages are
computed against this date, but the failure is currently loud rather than silent.

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

The member payloads are **encrypted** — length-preserving ciphertext with a
random per-file nonce and 45 bytes of framing, proven by two files whose
identical 10-byte `.img` plaintext shares no byte of its 63-byte block. FM 2023
shortlists are the same, so there is no plainer legacy format to target.

Do not pick this up looking for a decoding trick. There isn't one: it needs
SI's key, and taking that from `GameAssembly.dylib` is ruled out by
`LEGAL_NOTES.md:73` and §8.4, which make encryption the explicit stop condition
for this project. The route back in is asking SI for the format, not the binary.
If that ever happens, only the cipher is missing — everything around it is done.

**FM Genie Scout does export importable shortlists, and that is not a
counter-example.** A Genie-written file was examined: its members are encrypted
too, proven by a declared 21,232-byte JPEG member containing no `ff d8 ff`
anywhere. Genie holds SI's key. Its export capability is evidence that the key
*is* the route, not that another one exists — see `SHORTLIST_FORMAT.md` §4.

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
