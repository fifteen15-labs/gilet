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

**Whole people can be missing, not just links — SOLVED 4 August 2026.**
"Sometimes players are missing, like Mbappé" was not flakiness: aged saves
fold people who leave the loaded game world down to 30-byte **compact
entries** — a name-id pair and an entity object, embedded in the person
table between full records, with no record prefix for `scan_people` to find
(`SAVE_FORMAT.md` §6d-ter). Kylian Mbappé is one of 976 in the 2035 save
(day-one saves hold none, the 2030 Benchmark 180), and his uid appears
nowhere else in the frame — that entry *is* his storage.
`person::scan_compact` now reads them into `Save::people`, marked compact,
everything beyond name and identity honestly `None`. The UI deliberately
does not show them (owner's call, 4 August 2026): a row with no age, club,
attributes or contract answers no scouting question. None of them are
squad-referenced, so the residuals below are untouched by the fix.

**Residuals, in order of value:**

1. **Players at clubs outside the loaded leagues have no squad list at all.**
   In the Afan Lido save 6,585 of 27,483 contracted players resolve to no
   club (8,179 of 27,640 in the aged Ongoing.fm), and probing (diagnose
   example, "where do the orphans' squad lists
   hide") shows their eids and uids appear in *no* FF-marked count list in
   `game_db` — Brazilian, Argentine and MLS squads among them (Willian,
   Calleri, Acosta). The squad table only materialises for active divisions;
   FM must derive the rest from something else, likely the contract block,
   whose club link — if it exists — is not yet found. A naive scan of the
   ±384 bytes around the person record for known club eids matches only
   noise, and the contract anchor's post-eid u32 is not a club id either
   (Haaland 501, Sarr 550 — no relation to Man City/Spurs ids). One lead:
   the `used_player_data.dat` member holds one entry per player
   (`[eid u32][tag u8][year 0x07E9][packed bytes]` — Willian and Calleri
   both present exactly once), so its packed payload may carry the
   registration club. Next: decode that member, or diff two same-club
   orphans' full records against a third-club orphan for a shared u32.
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

## 2. Attribute names — SOLVED, all 54 named

The last five — the hidden attributes at 41, 44, 47, 48, 49 — fell on
3 August 2026 to Haaland's pre-game-editor sheet: Dirtiness 9,
Consistency 16, Important Matches 14, Injury Proneness 10, Versatility 7,
five distinct values matching his decoded block one-for-one (single
possible assignment; every visible attribute on the same sheet matched
its named index exactly). The UI shows them as their own "Hidden" group.
The history of the first 49:

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

## 3b. Staff attributes — SOLVED 3 August 2026

**The 54-value block is the editor's 52-item attribute list at slots 2-53.**
Three separate errors had to be undone at once, which is why every earlier fit
failed; the history is kept below because each dead end rules something out.

1. **The block belongs to the object at person eid − 1, not eid n.** A person
   carries two entity objects — the non-player one first, then the person one
   inside their name record (Sterling: 8401 then 8402). The staff sheet is on
   the *first*. Every fit until now compared person X's editor sheet against
   person X+1's block.
2. **The five u16s are `[home rep, current rep, world rep, CA, PA]` and they
   are the database's own numbers.** On the right object they match the editor
   exactly, five values each: Fradley 6350 / 7000 / 5500 / 140 / 150 against
   his editor Game Reputations 7000 (Current), 6350 (Home), 5500 (World), CA
   140, PA 150; Nikolić 6250 / 6500 / 4500 / 130 / 145 against 6500 / 6250 /
   4500, CA 130, PA 145. **Reputation is not regenerated at career start** —
   the earlier conclusion was drawn from the neighbouring person's object. The
   field order is home-first, which the editor prints second.
3. **The fields are not at a fixed stride from the tag byte.** Nikolić's object
   carries a preamble of 8-byte rows between tag and fields, so `at + 20 (+2)`
   lands in the middle of it and reads garbage. Locate the fields by their own
   signature — three multiples of 50 followed by two 0-200 values — not by
   offset.

**The map.** Slot = list index + 1, so items 1-52 occupy slots 2-53. `s0` is a
small enum (14 values seen; 22, 33, 4, 30, 32 the commonest) and `s1` ≡ 12 in
97.7% of blocks — a two-byte header, exactly as the item count implies.

| slots | items | storage |
|---|---|---|
| 2-27 | 1-26 — Attacking through Width, the tendency half | **raw 1-20** |
| 28-53 | 27-52 — Coaching through Coaching Set Pieces, the coaching and knowledge half | **raw × 5** |

with a small per-person day-one drift of 0 or −2 on the ×5 half. **Not ×4, and
not the controlled column** — it is the editor's *raw* value throughout.

**Verification.** Nikolić (block at eid 5155): thirteen raw-exact in slots
2-27 — Attacking 9, Directness 14, Authority 17, Trigger Press 16, Youngsters
11, Buying Players 10, Mind Games 12, Depth 10, Flexibility 12, Hardness of
training 15, Squad rotation 8, Tempo 11, Width 10 — and eleven more within 2 of
raw × 5 in slots 28-53. Fradley (block at eid 20129): all fifteen of his
non-blank items land within 2, including Coaching Fitness 1 → 5, Coaching
Attacking 5 → 25, Coaching Possession 6 → 30 and Coaching Technical 16 → 80.
The twenty-four items blank in the database hold generated values, as the
Hutton and Gerig sheets predicted.

**It parses.** `staff.rs` scans the objects, locates the five u16s by
signature rather than stride, converts the coaching half back by rounding to
nearest, and binds each sheet to the person one eid up. `Person::staff` exposes
it. With rounding the recovery is exact, not approximate — every non-blank item
on both editor sheets reads back byte for byte, and
`real_save::staff_sheets_match_the_editor` asserts 34 of them.

**4 August 2026 — the header requirement was hiding 40% of the sheets.** The
sheet-bearing triple is nearly always the *previous* person's own identity
(18,202 of 18,247 exact eid+uid matches on Day One), and identities are often
written with no object header at all. `scan_staff` required the header, so
Arne Slot, Arteta and ~7,400 others showed no sheet; anchoring on the triple
(header **or** the identity's three zero bytes) lifts Day One from 10,800 to
18,245 bound sheets. The headerless shape also slipped past the shadow drop —
Verberne bound as eid 526592 (2057 << 8) — closed with a value-shape test in
`scan_triples` (both ids end in a zero byte, next offset reads them shifted
back). Both locked by `real_save` assertions on Slot and Verberne.

**What this retires:** the ×4 scale, the "controlled column" reading, "career
start rewrites reputation", "the block is generated for everyone", and the
Nikolić negative — his sheet *is* in the save, one object earlier.

## 3b (history). Staff attributes — structure found, ids unmapped

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
3. **The 1-100 run at ~+458 is the tail of a 54-value block** — resolved in
   structure on 3 August 2026, still open in semantics; see below.

**3 August 2026: pure staff carry a 54-value attribute block after their
person object** (`staffscan`, `staffobj`, `dumpat` examples; Career (v02),
a 26.0.0-DB save dated 26 Oct 2025, Emery's record at `0x4409931`):

1. **The layout.** At the record's tail sits an object header
   `02 40 10 [u32 flags] [eid 135][uid 5193][uid 5193]` — the person object
   itself, uid = the staff DB id efem.club uses — then a tag byte `01`,
   five u16s reading 5000/5000/1500/125/125, then 62 bytes ending in the
   record's `8×FF` terminator. The last 54 of those bytes read as an
   attribute block: counting 54 back from the `FF` run lands on value 22 =
   Emery's published Adaptability, exactly. Haaland's *non-player* object
   (external, found by searching his doubled non-player uid 29179299) has
   the same shape with tag `02`, triple 5350/5350/2750, and a 54-block
   still sitting on display×5 multiples in this fresh save.
2. **Slots 30-53 are the old "+458 run"** — 24 values for the 24 published
   attributes, and the counts matching is the strongest alignment evidence
   so far.
3. **The values are not the published values, anywhere, in any scale.** A
   frame-wide order-blind scan (sorted-multiset distance, `staffscan`) for
   Emery's efem 26.0.0 sheet — whole 24, the 11 coaching-screen values, the
   mental five, the knowledge six; raw 1-100 and display 1-20 — found zero
   matches in every frame of the archive, on the freshest save, where drift
   cannot be the excuse. The two stray 5-value near-hits were in other
   people's blocks.
4. **The transform is systematic and open.** Emery's slots 30-53 read
   63/68/59/63/10/68/59/55/33/63/50/59/80/60/10/60/25/40/65/10/45/43/61/35.
   Low published values survive exactly (10, 12, 13, 22, 33 all present);
   high ones compress (Discipline 100 → max 80, Tactical 92 → ~68). Rank
   order is roughly preserved, absolute values are not — a concave squash,
   not a linear rescale. Between the original Career.fm reading and
   Career (v02) the values crept up +1/+2 with the 10s pinned, so the block
   is live state on top of whatever the transform is.

**Later on 3 August 2026 — mass extraction, block anatomy, and a hard
negative on published sheets** (`staffmap` example dumps every extractable
block as CSV with owners: 259 in Career (v02), 683 in Port Talbot.fm —
which turned out to be a 26.2.0-DB save, giving a version-matched pair):

**Anatomy, version-independent (942 blocks):** slot 1 ≡ 12 in every block
(structure, not data); slots 2-27 minus 13/15 are twenty-four 1-20
values, uncorrelated with the 1-100 region (max r 0.30 over 259 — not a
display mirror); slot 13 ranges 1-9; slots 0/15/28/29 are oddballs;
slots 30-42 are a **stable 13-slot 1-100 array** — the same person reads
near-identical values across both saves (Emery ±2, Zidane exact while
unemployed); slots 43-53 are a **mutable tail** whose values genuinely
reshuffle between saves (Zidane s44 reads 70 in one save, 30 in the
other, with his front array untouched) — list-like state, not a fixed
array. The 2-August "+458 run" framing and today's earlier "slots 30-53
are the 24 attributes" framing are both dead: only 13 slots are stable.

**The negative, worth not re-treading:** the block does not hold the
efem.club attribute sheet under *any* slot assignment. Tested by
one-to-one assignment fits (L1 and correlation) with eleven fetched
sheets — Emery, Zidane, Beukenkamp, Mancini, Vulcano, Ibáñez, Le Bris,
Shin, Caulfield on the 26.0.0 save, then Emery, Zidane, Mancini,
Caulfield, Maldini, Allegri, Raúl, Hoegee **version-matched on the
26.2.0 save**. Apparent locks at small n (GK slots at n=2, Judging Staff
Ability at slot 42 with r=1.00 at n=4) all collapsed by n=8; the
survivors of the early fits were low-value constants matching low GK
numbers, and generic-high slots matching whatever was high. Conclusion:
either the save stores staff numbers efem does not publish (recomputed
live values on another basis), or the real sheet lives elsewhere — the
id→value row list in the record body is back to being the prime
candidate for the coaching data.

Useful side-facts confirmed on the way: the save's staff **uid is
exactly the efem URL id** (`/staff/5193-Unai-Emery`); efem also lists
Reputation and 1-100 personality values per staff; and DB revisions
moved staff numbers hard between 26.0.0 and 26.2.0 (Emery's Analysing
Data 13 → 60), so any future ground truth must be version-matched.

**Later still on 3 August 2026 — the editor cracks the scale and the
personality slots** (pre-game editor screenshots of Guardiola against his
record in Port Talbot (v03).fm at `0x55a1439`):

1. **All eight hidden personality slots are named** (see `SAVE_FORMAT.md`
   §"hidden personality run") — Pep's editor sheet matches his run exactly,
   and the old slot-1 = Loyalty claim was wrong (slot 1 is Ambition, Loyalty
   is slot 2). `Person::loyalty()` was reading the wrong index; fixed, with
   getters for all eight now.
2. **Staff attributes are stored on a 1-80 scale: editor value × 4.** Pep's
   editor 20s sit in his block as 79/79/75/75 (in-save drift −1..−5), his
   Coaching Defending/Possession 17 → 68 appear as 69/69, Working With
   Youngsters 16 → 64 exactly, Authority 15 → 60 exactly, Mind Games
   16 → 64 at slot 42 (65). This dissolves the "compression transform":
   efem publishes editor×5, the save stores editor×4, so every earlier fit
   was off by exactly 0.8 — the ratio the first Emery fit actually measured.
3. **The editor's flat attribute list is 52 items — the 54-slot block is
   that list.** The mixed scales inside the block follow the editor's
   weighting column: CA-weighted attributes (Judgement 4, Motivating 4,
   the coaching set at 3…) store fine-grained 1-80 in slots 30-53;
   unweighted ones (tactical style: Directness, Tempo, Width…) store raw
   1-20 in slots 2-27.
4. **The five u16s after the object tag: the pair is staff CA/PA** on
   0-200 — Pep 149/160, Emery 125/125, Zidane 109/130, Beukenkamp (free
   agent) 100/120 — and the triple in front (Pep 7358/7358/6680, Emery
   5000/5000/1500) is reputation-shaped and back on the table now that
   the pair anchors the layout.
5. The object header's third byte varies more than `10|18` — Pep's reads
   `02 40 1a`. `staffmap` accepts `1a` now; treat the byte as flags.

**And later again on 3 August 2026 — Mancisidor (GK coach) editor sheet
confirms the reputation triple and lands the first seven slot locks:**

1. **The five u16s are one structure across staff and players, exactly
   quantized — but their meaning is NOT settled.** Layout:
   `[u16 A][u16 B][u16 C][u16 D][u16 E]` with **D ≡ B/50** (holds for
   22,095 people in Day One.fm) and A/B/C always multiples of 50 —
   i.e. three 0-10000 values that are raw-0-200 × 50, then the raw B,
   then a second 0-200 value E. For **staff** the triple shape-matches
   the editor's "Game Reputations (Scaled Up To 10000)" line
   (Mancisidor: editor 8000/8000/6250, day-one save 6750/6750/4250 —
   same [current, home, world] shape, not equal; some career-start
   recalibration). For **players** the reputation label is
   **contradicted outright** on the same day-one save: Wissa reads
   9300 and a CA-8 lower-league veteran 9200 while Mbappé reads 6800,
   Haaland 5250 and Lamine Yamal 2750 — no reputation concept orders
   these people that way (record-anchored reads, not the noisy sweep).
   Whatever A/B/C/D/E measure, it is per-universe state with a strong
   loaded-league flavour, and **must not ship labeled as reputation**.
   E (Pep 160, Mancisidor 147, Haaland 120, van Dijk 152) is also
   unidentified.
2. **The save stores the editor's *controlled* values, not raw DB
   values** — the effective in-game numbers the editor shows in its
   "controlled attribute" column (Mancisidor's raw-0 Coaching Possession
   → controlled 10; his raw-17 GK Coaching → controlled 20). Storage is
   **controlled × 4** on the 1-100 slots, with a further small per-save
   rebase for most attributes (Pep's controlled-20s sit at 69-79 on day
   one, not 80).
3. **Region anatomy, corrected by the day-one cross-check.** Only
   **slots 30-42 are a stable, DB-derived array** — Pep's thirteen
   values are byte-identical across two different careers bar one slot.
   The 1-20 region (s2-27) and the tail (s43-53) **reorder per save**,
   so any slot lock made in one save is save-local there. Earlier locks
   in those regions (Tempo 8, JSA 9, Set Pieces 22, Authority 27,
   JPP 43) are retracted as fixed positions — right values, save-local
   ordering.
4. **Locks that survive cross-save and cross-person:** slot 36 =
   **Coaching Possession** (day-one exact for both people, including
   the controlled-10 → 40 signature), slot 34 ≈ Negotiating, slot 15 ≈
   Motivating, slot 1 ≡ 12. Coaching Goalkeeping is *not* in the stable
   front — a GK coach's defining attribute lives in the per-save tail.

**Front-13 fit, 3 August 2026 evening** (Day One.fm, Pep + Mancisidor,
per-person scale fitted — values sit at ~0.87-0.90 of editor
controlled×4, plausibly CA-linked): **near-locks s31 = Judging Player
Potential (d 1.7), s32 = Determination (d 1.2), s33 = Judging Player
Ability (d 4.1), s41 = People Management (d 2.4)** on top of the s34/s36
locks. Left degenerate: {s30, s35, s37, s39, s40} across {Motivating,
Tactics, Coaching Attacking/Technical/Tactical, GK Coaching} — and
Mancisidor reads 50-60 at s30/s37/s40 where his editor coaching values
are 0, so **front membership may itself be role-dependent**. The earlier
s15 ≈ Motivating guess is deprecated (Mancisidor s15 = 85 exceeds his
scaled maximum; s15 is something else). Three contrasting editor sheets
(a physio, a data analyst, a fitness or assistant coach) break the
cluster and settle membership.

**Two more editor sheets, 3 August 2026 — a data analyst and a physio.** The
front-13 fit asked for exactly these contrasts. One of the two people is in no
local save, and the other turns out to carry no staff block at all, so the
cluster is not broken yet — but the pair settles the five-u16 run.

Ground truth banked (Editor 26; General, Tactical and Non Tactical pages):

- **Daniel Fradley** — CA 140, PA 150, Recommended CA 143 (Data Analyst /
  Performance Analyst / Recruitment Analyst), Coaching Style Technical, Current
  Reputation 140, Home 127, World 110, i.e. "Game Reputations (Scaled Up To
  10000)" 7000 / 6350 / 5500. Non-tactical in editor order: Buying Players 0,
  Eccentricity 0, Hardness of Training 0, Judging Player Ability 12, Judging
  Player Potential 14, Judging Player Data 16, Judging Staff Ability 0,
  Authority 0, People Management 12, Mind Games 0, Motivating 10, Versatility 0,
  Squad Rotation 0, Working With Youngsters 12, Coaching Attacking 5, Coaching
  Defending 5, Coaching Fitness 1, Coaching Goalkeeping 0, Coaching Possession
  6, Coaching Tactical 16, Coaching Technical 16, Coaching Set Pieces 12,
  Business 0, Negotiating 0, Interference 0, Patience 0, Resources 0,
  Physiotherapy 0, Sports Science 0. Tactical: Attacking 0, Depth 0,
  Determination 13, Directness 0, Physicality Of Play 0, Flexibility 0, Line Of
  Engagement 0, Tactical Knowledge 16, Tempo 0, Width 0.
- **Steffen Lutz** — CA 140, PA 148, Recommended CA 180 (Physio), Coaching Style
  Fitness, Current Reputation 140, Home 140, World 110, Game Reputations
  7000 / 7000 / 5500. Every tactical and non-tactical value 0 **except Working
  With Youngsters 15 and Physiotherapy 18** — two non-zero slots out of
  thirty-nine, the sparsest fingerprint available and the one that would settle
  membership in a single fit.

1. **Lutz is in none of the twelve local saves** (`findname` example, substring
   search over parsed people): the only Lutzes anywhere are Lutz Pfannenstiel
   and Charlie Ethan Lutz. His sheet is banked for a save that loads German
   staff; the physio contrast is still outstanding.
2. **Fradley is in three** — Day One.fm `0x534e87e`, Port Talbot (v03).fm
   `0x6720ead`, Ongoing.fm `0x57fea39` — one uid (72049878) across three
   independent careers, so his fields can be split into DB-derived and live.
3. **He has no staff attribute block.** No block among the 1,975 `staffmap`
   extracts from Day One.fm carries his eid, his triple or his pair, and a
   multiset fit of his fifteen non-zero editor values at ×4 and ×5 over all
   1,975 tops out at 12 of 15 on unrelated people — noise, because small ×5
   values are everywhere. His object at `0x534e8d3` is followed by eight filler
   bytes, a fifteen-byte 1-20 run and a fifty-four-byte 1-100 run, and that run
   is not a player block either: on this fresh save every real one is exact
   display×5 (Sterling's is 65/75/60/…), while his mixes ×5 with ×5+1 and holds
   his editor sheet at no scale — his four 16s need four 81s and there is one.
   So **a player-coach's non-player sheet is not adjacent to his person
   record**. That is a new place to look rather than a new dead end: it is the
   same "second object elsewhere" arrangement Haaland's non-player block has.
4. **The five u16s: D and E are exactly what `ability.rs` already reads as CA
   and PA.** `CA_BACK`/`PA_BACK` (39 and 37 bytes back from a block) land on D
   and E. Sterling's block at `0x49d3e31` is preceded by an object at
   `0x49d3df0` holding eid 8401 / uid 28054286 — not his person pair
   8402 / 28054109 but the consecutive-eid second object — with fields
   6500/7250/6000/139/180, and 139/180 is the CA/PA the parser reports for him.
   The triple and the pair are one structure on players as well as staff, and
   the pair is settled: D = Current Ability, E = Potential Ability.
5. **The triple is the reputation line by structure and unrecoverable by
   value.** Across the 1,975 day-one staff blocks: t3 is the smallest of the
   three in 92.7%, t1 == t2 in 82.9%, all three are multiples of 50 in 68.7%.
   Both editor sheets have that shape — world lowest, current == home for Lutz.
   The numbers never survive career start: Fradley's editor 7000/6350/5500
   against a day-one 5900/6150/4650, Mancisidor's 8000/8000/6250 against
   6750/6750/4250, Haaland's 9350/9300/9300 against 5250/5250/2250. Fradley is
   also the only staff member seen so far whose editor current (140) and home
   (127) differ, and his save reads t1 118 < t2 123 — but all three of his
   values moved, so that is not evidence of a swapped order. (Partly overturned
   an hour later — see the Hutton match below, where current and home survive
   career start byte-exact.) t2 == CA×50 holds in only 40.8% of the population,
   so `D ≡ B/50` stays an initialisation artifact, not a derivation.
6. **`staffmap`'s census undercounts.** Its 8×FF anchor requires the block to
   end at the record terminator. Fradley's object is followed by more record
   body and only a 5×FF run, so he is extracted as nothing — as is every record
   shaped like his. Relax the anchor before trusting the denominator of any
   population fit.

**Three more editor sheets the same afternoon — and the reason every fit has
failed.** Picked from Day One.fm by their *save* fronts: near one-hot blocks,
each spiking a different unmapped slot, exactly the contrast the front-13 fit
wanted. The picks were self-defeating, and finding out why is the result.

| person | editor CA/PA | editor reps (cur/home/world ×50) | save eid/uid | save p1/p2 | save t1/t2/t3 | save front s30-42 |
|---|---|---|---|---|---|---|
| Tom Hutton | 100 / **-7** | 5000 / 5000 / 1750 | 33829 / 2000219276 | **100** / 110 | **5000 / 5000** / 1250 | 58,58,83,98,5,26,26,26,26,100,26,26,95 |
| Hansueli Gerig | 80 / 95 | 4000 / 4000 / 2000 | 47036 / 2000432645 | 100 / 120 | 4896 / 4896 / 2966 | 36,36,100,92,5,36,36,36,36,100,36,36,70 |
| Joel Cornelli | 81 / 89 | 1150 / 1150 / 500 | 1389 / 308959 | 135 / 155 | 6000 / 5500 / 3750 | 64,54,29,81,5,64,24,100,50,81,14,91,70 |

Their editor sheets, in full: **Gerig** — every one of the thirty-nine tactical
and non-tactical values 0. **Hutton** — all 0 except Working With Youngsters 12.
**Cornelli** — all 0 except Directness 5, Tempo 4, Width 6.

1. **`staffmap`'s owner column is right, at least for records of this shape.**
   Hutton's and Gerig's blocks carry an object whose eid *and* uid equal the
   person's own pair as `bind_identities` resolved it — 33829 / 2000219276 and
   47036 / 2000432645. Attribution by containing record survives the check.
   (Cornelli's identity is unbound, `eid: None`, so his row stays unverified and
   his wild mismatch may just be the wrong person.)
2. **First exact editor-to-save reputation hit.** Hutton's DB Game Reputations
   5000 (Current) and 5000 (Home) are in the save byte-for-byte as t1 and t2,
   with his DB CA 100 sitting in p1. So career start does **not** always rewrite
   the triple — the field really is that line. His **world value misses**
   (editor 1750, save 1250), and Gerig's and Cornelli's miss on all three. One
   clean survivor in four staff checked; whatever chooses between preserved and
   recalculated is unknown, and until it is known nothing here is shippable.
3. **PA in the database can be a negative random range.** Hutton's editor PA is
   **-7**; the save holds 110. So p2 is a value resolved at career start, not a
   DB copy — which also explains Fradley (editor 150, save 138) without needing
   a version skew.
4. **The 54-block is generated, not copied — and that is why every fit has
   failed.** Gerig's DB row is *empty*, all thirty-nine values 0, and his save
   block is dense: a flat floor of 36 with spikes of 100/92/70. Hutton's row has
   one non-zero value and his block is a flat 26 with spikes of 100/98/95/83/58.
   The game synthesises a staff sheet at career start from CA and role for
   anyone whose DB row is blank. Fitting the block against the editor's **raw**
   column is therefore hopeless for most staff, and every past failure —
   the eleven efem sheets, the version-matched eight, today's Fradley multiset —
   is explained without needing a wrong slot map.
5. **Choosing candidates by their save block selects against usable ground
   truth.** A near one-hot front is the signature of a blank DB row: flat
   generated filler plus a couple of generated spikes. Rich raw rows (Pep,
   Emery, Mancisidor, Fradley) are the only people whose sheets can name a slot,
   and they are exactly the people whose fronts are *not* extreme.
6. **The editor's "All Attributes" page is now the critical unknown.** It is the
   derived/controlled column — the thing the save actually stores — and it is
   present in the sidebar for Gerig and Cornelli. One capture of that page for a
   person already located in Day One.fm would give a direct slot-to-name key
   with no scale guessing and no dependence on a non-blank DB row.

**The "All Attributes" page, captured (Tom Hutton, 3 August 2026).** It has
five columns — attribute, value, weighting, controlled attribute, controlled
attribute difference — and reports **52 items**. It answers the question it was
fetched for, in the negative.

1. **The controlled column is not what the save stores.** Hutton's entire
   controlled sheet is 0 or 1 except Youngsters 12 (his one non-zero raw value,
   passed through unchanged). His save block is dense: a flat floor of 26
   repeated seven times with spikes of 100/98/95/83/60/58. Neither the raw
   column nor the controlled column produces that. **The block is generated at
   career start for staff whose database row is blank, and no editor page will
   ever name its slots for such a person.** The "controlled ×4" reading from
   Pep survives only because Pep's raw row is rich, so his controlled column is
   near his raw one.
2. **The CA weighting table, read directly** (previously guessed, and guessed
   wrong — the doc had Judgement at 4 and the coaching set at 3):

   | weight | attributes |
   |---|---|
   | 4 | Coaching Attacking, Coaching Defending, Coaching Possession, Coaching Technical, Coaching Tactical |
   | 3 | Judgement, Judging Potential |
   | 2 | Determination, Motivating |
   | 1 | Youngsters, People Management, Tactics, Coaching Fitness, Negotiating, Judging Staff Ability |
   | 0 | everything else — including Coaching Goalkeeping, Coaching GK Handling, Coaching GK Distribution, Coaching Set Pieces, Physiotherapy, Sports Science, Analysing Data, Versatility, Eccentricity, Dirtiness allowance and every tactical tendency |

   **Caveat: this may be role-scoped.** Hutton's Recommended CA Job Role is
   *Coach*, and a goalkeeping coach weighting Coaching Goalkeeping at 0 makes no
   sense, so the column is probably computed for the person's recommended role
   rather than being a global table. Confirm on a second person with a different
   role before relying on it.
3. **The list order, 49 of 52 rows** (top to bottom, editor's own spellings):
   Attacking, Business, Coaching Technique, Directness, Authority, Free Roles,
   Interference, Marking, Offside, Patience, Trigger Press, Resources,
   Youngsters, Determination, Buying Players, Mind Games, Sitting Back, User Of
   Play-Maker, Use Of Subs, Hardness of training, Squad rotation, Tempo, Width,
   Coaching, Coaching Goakeeping, Judgement, Judging Potential, People
   Management, Motivating, Physiotherapy, Tactics, Coaching Attacking, Coaching
   Defending, Coaching Fitness, Coaching Possession, Coaching Technical,
   Coaching Tactical, Dirtiness allowance, Coaching GK Handling, Coaching GK
   Distribution, Versatility, Analysing Data, **(unnamed)**, **(unnamed)**,
   Sports Science, Eccentricity, Negotiating, Judging Staff Ability, Coaching
   Set Pieces. Three rows below Coaching Set Pieces were not captured, and
   **two rows the editor itself leaves blank** — so even a perfect slot map
   tops out at 50 of 52 names from this source.
4. **The tidy "s0/s1 are a header, s2-s53 are the 52 items" mapping does not
   hold.** s1 ≡ 12 in 1,975 of 1,975 blocks (structural, re-confirmed), but s0
   is not constant — 22 in 33%, 4 in 18%, 33 in 17%, discrete enough to be an
   enum, not a header byte. And the list order does not map positionally:
   Hutton's Youngsters 12 is not at the slot its list position would give
   (s14 = 20), nor are Cornelli's Directness 5, Tempo 4 and Width 6 at theirs
   (s5 = 1, s23 = 19, s24 = 13).

**Marko Nikolić, a populated row, fitted — and it fails (3 August 2026).** A
working manager, DB row full, in Day One.fm, block already extracted. This is
the test the whole programme was waiting for.

Editor: CA 130, PA 145, Recommended CA 136 (Manager), Coaching Style General,
Game Reputations 6500 (Current) / 6250 (Home) / 4500 (World). Save block at
`0x4718ea8`: p1/p2 **128 / 145**, triple 6000/6000/5700, low region s2-27
`12,3,11,12,14,14,6,9,18,6,11,5,13,65,12,12,9,13,7,11,15,10,13,12,9,8`, high
region s30-53 `60,70,60,60,35,60,60,65,15,65,50,60,40,60,75,55,25,55,75,20,30,
60,55,25`.

1. **PA lands exactly** (editor 145, save 145) and CA is within two (130 → 128),
   which is the third independent confirmation of D = CA, E = PA.
2. **His sheet is not in his block at any scale.** Controlled ×4: 6 of 52
   values present — dead. Controlled ×5: 20 of 52, and that is inflated by 60
   and 65 being the commonest block values anyway; the block holds two 65s where
   the sheet needs five. **His two best weighted attributes have no home at all**
   — Motivating 16 and Coaching Defending 16 want 64 (×4) or 80 (×5) and the
   block's high region tops out at 75, with neither value anywhere in the 54.
   Raw ×1 against the low region scores 18 of 28, at chance for a region already
   full of 9-15s, and his two most distinctive raw values, **Authority 17 and
   Trigger Press 16, do not appear** (low region maxes at 18, with no 17 and
   no 16).
3. **So the ×4 reading is down to one person.** It came from Pep, whose hits
   (WWY 16 → 64, Authority 15 → 60) are values common enough in a block to land
   by accident, and it does not reproduce on a second rich row in a
   version-matched, day-one, own-record-anchored fit. Treat "controlled ×4" as
   unproven, not established.
4. **The weighting column is role-scoped — confirmed.** Nikolić (Manager) and
   Hutton (Coach) disagree on almost every weighted attribute: Judgement 4 vs 3,
   People Management 4 vs 1, Motivating 4 vs 2, Tactics 4 vs 1, the coaching set
   3 vs 4, Coaching Fitness 0 vs 1, Negotiating and Judging Staff Ability 3 vs 1.
   The column is computed for the person's Recommended CA job role, so it is not
   a global table and cannot be used to predict which slots are stored coarsely.
5. **The 52-item list is now complete**, Nikolić's capture supplying the three
   rows Hutton's cut off (20 Depth, 21 Fluidity, 22 Flexibility) and confirming
   52 Coaching Set Pieces as the last:

   1 Attacking, 2 Business, 3 Coaching Technique, 4 Directness, 5 Authority,
   6 Free Roles, 7 Interference, 8 Marking, 9 Offside, 10 Patience,
   11 Trigger Press, 12 Resources, 13 Youngsters, 14 Determination,
   15 Buying Players, 16 Mind Games, 17 Sitting Back, 18 User Of Play-Maker,
   19 Use Of Subs, 20 Depth, 21 Fluidity, 22 Flexibility,
   23 Hardness of training, 24 Squad rotation, 25 Tempo, 26 Width, 27 Coaching,
   28 Coaching Goakeeping, 29 Judgement, 30 Judging Potential,
   31 People Management, 32 Motivating, 33 Physiotherapy, 34 Tactics,
   35 Coaching Attacking, 36 Coaching Defending, 37 Coaching Fitness,
   38 Coaching Possession, 39 Coaching Technical, 40 Coaching Tactical,
   41 Dirtiness allowance, 42 Coaching GK Handling, 43 Coaching GK Distribution,
   44 Versatility, 45 Analysing Data, 46 **(unnamed)**, 47 **(unnamed)**,
   48 Sports Science, 49 Eccentricity, 50 Negotiating, 51 Judging Staff Ability,
   52 Coaching Set Pieces.

6. **The attribution caveat that now matters most.** Nikolić's identity is
   unbound — `findname` gives him `eid: None` at `0x4718df3` — so his block is
   owned by `staffmap`'s span heuristic, not by a matching eid/uid pair. The
   two people whose ownership *was* verified that way (Hutton, Gerig) are
   exactly the two whose blocks are provably generated filler. Until
   `bind_identities` gets its staff pass, every fit rests on an unverified
   attribution, and that is the cheapest thing left to fix.

**Staff identities bind — and the reason they did not was a scanner bug
(3 August 2026).** `bind_identities` was blamed for dropping staff because
their eids sit outside the ascending chain. That is true, but it was not what
lost them.

1. **The shadow hit.** `scan_triples` accepts an `[eid][uid][uid]` triple
   preceded by three zero bytes, then steps twelve bytes past it. An entity
   object header ends in zero bytes, so reading the eid *one byte early* also
   passes: the short read gives `eid << 8` with the repeated uid still lining
   up. That shadow is accepted whenever `eid << 8` stays under `MAX_EID`
   (3,000,000) — i.e. **below eid 11,718** — and consuming its twelve bytes
   hid the real block behind it. Nikolić (5156), Cornelli (1389) and
   Pfannenstiel (1858) all sit in that band. Fradley (20130) and Hutton (33829)
   bound fine, because their shifted eids overflow the bound. The symptom
   looked like "staff are out of order"; the cause was an off-by-one that only
   bites low eids.
2. **The fix is one lookahead.** A shadow sits exactly one byte in front of the
   block it hides, so a hit is dropped when the very next offset carries a
   triple proven by an object header (`[type 00-02][0x40]`, seven bytes back).
   Dropping it rather than merely stepping onto it also keeps it out of the
   ascending chain, where it could otherwise beat its own block. Cost is one
   extra header test per hit: measured against the pre-change parser back to
   back on the same (heavily loaded) machine, 19.2/14.0/16.9s against
   20.7/19.2/19.8s — no regression.
3. **The header cannot simply be required of every candidate.** Tried: it
   leaves 26,089 people unnamed against 1,095, so plenty of real identity
   blocks are written without one. It is a tie-breaker, not a filter.
4. **The out-of-order pass then takes the leftovers by shape** — inside the
   record, within 512 bytes of its prefix (`IDENTITY_WINDOW`, from the `idgap`
   probe: median 145, p99 402, 99.80% inside 512), ids not already bound to
   somebody, first such block in the record. Anything else keeps `None`.
5. **Result on Day One.fm: unbound people fall from 5,530 to 1,095**, and all
   three staff bind to exactly the ids `staffmap` had guessed from their spans
   — Nikolić 5156 / 5790125, Cornelli 1389 / 308959, Pfannenstiel 1858 /
   434431. So the block attributions those fits rested on were right after all,
   and the Nikolić negative stands: **his sheet really is not in his block.**
   van Dijk, Fradley and Hutton are unchanged.

Covered by `person.rs` tests: the shadow at `eid << 8` must not win, an
out-of-order eid still names its record, a block deeper than the window is
refused, and an already-bound id is never reused.

**Where to go next:**

0. **Get General + All Attributes for a staff member with a *populated*
   database row who is also in Day One.fm.** That is the only remaining route
   to a slot map: blank-row staff have generated blocks that encode nothing.
   Working managers are the safe bet — `staffmap`'s owner column has Marko
   Nikolić, Sergej Jakirović, Marinus Dijkhuizen, Nikos Nioplias and Jiří
   Jarošík, all real coaches with real sheets, all with blocks already
   extracted. Two of those settle both the slot map and whether the weighting
   column is role-scoped.
1. **Map the stable front first** (s30-42 + s15 + s28/s29): thirteen-ish
   DB-derived slots, fittable across all 1,976 day-one blocks at once —
   population statistics against role expectations (every manager high,
   only physios high, …) now work because the front does not shuffle.
2. **Decode the shuffled regions as what they probably are — a list.**
   The per-save reordering of s2-27 and s43-53 says those bytes are a
   serialized id→value list whose order was fixed at career creation,
   not a fixed array. Find the id table that orders them (the `10 1d 04
   .. 04` preamble bytes in front of the run are the first suspects).
3. **SETTLED, twice over (Haaland's editor General page, 3 Aug 2026):**
   the editor says Game Reputations 9350/9300/9300 (raw 187/186/186);
   the day-one save's run for him reads 5250/5250/2250 — **the DB's
   reputation numbers are not in the save**. (Refined by the Fradley and
   Lutz sheets above: the run *is* the reputation line by structure —
   world lowest 92.7%, current == home 82.9% — but career start rewrites
   every value, so the conclusion for shipping is unchanged. D and E in
   the same run are Current and Potential Ability.) And the editor's numbers appear
   **nowhere in the frame** (u16-triple and raw-byte scans, zero hits,
   `repfind` example) — FM regenerates reputation per career with values
   that match nothing in the DB. Player reputation is therefore
   unrecoverable without in-game-editor numbers read from a live save.
   The same editor session also killed transfer value for good: his DB
   Transfer Value £120,000,000 is absent near his record in every /10^k
   encoding (`valuefind` example; the only frame-wide 120M hits are
   competition prize money before the person table). And it confirmed
   the personality slot map on a second person — six exact matches
   (Loyalty 15, Pressure 18, Professionalism 17, Sportsmanship 10,
   Temperament 12, Controversy 11 against his run
   `[16,20,15,18,17,10,12,11]`).
2. **Decode the row list ids.** Emery's rows hold 100/90/75/70/65/…
   values under u32 ids with type tags (`08 00 03 01`, `03 00 01 03`,
   `0a 00 03 01`, `00 00 03 02`, `03 53 01 ff`). Group rows by type tag
   across the 942-block corpus and match value distributions per tag —
   knowledge rows are proven (nation ids, 100 = Complete); one of the
   other tag families may be the coaching sheet keyed by attribute id.
3. **Name the mutable tail.** Whatever slots 43-53 hold changes between
   saves for the same person — diff one person across the four dated
   saves to see if it tracks employment, form or morale-like state.
4. **The second identity.** On later saves Emery carries two consecutive
   objects (eid 144/uid 5193, then eid 145/uid 5227) — person and
   non-player. Which of the two carries the block there needs one dump.
5. **DONE — `bind_identities` second pass** (see the shadow-hit section
   above). Pure-staff records get their eid; 4,435 more people are named on
   a day-one save.
6. **Find a player-coach's non-player object.** Fradley (§ the two editor
   sheets above) has a full editor coaching sheet and no staff block near
   his record, so his non-player numbers live in a second object with a
   different uid, the way Haaland's do. Sweep for a `[eid][uid][uid]`
   object whose eid neighbours his person eid 20130 in Day One.fm and
   check the 54 bytes after its five u16s against his sheet — fifteen
   non-zero values at ×4 and ×5 is a strong enough signature to confirm
   or kill it in one pass.
7. **Relax `staffmap`'s 8×FF anchor** before any further population fit:
   it silently drops every record whose object is followed by more record
   body, Fradley's among them.

The `02 40 1?` header's five u16s (after the tag byte) read Emery
5000/5000/1500 + 125/125, Zidane 4600/5450/3750 + 109/130, Beukenkamp
(a free agent) 4000/4000/400 + 100/120 — triple-plus-pair, shape and
magnitudes compatible with a reputation triple (efem lists Emery
Reputation 86) and worth testing when reputation is hunted properly.

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

## 3c. Staff employer — day-one SOLVED, aged saves partial

**Managers bind everywhere** (4 August 2026, `backroom.rs`,
`SAVE_FORMAT.md` §6h): the per-club roster table's manager slot, validated
against the club's doubled uid — Slot/Arteta/Guardiola exact on day one,
Iraola at Liverpool in a 2030 career, Klopp correctly unemployed there.

**The rest of the backroom binds on day-one saves** (5 August 2026): the
club record's body carries count-prefixed ascending person-eid lists —
FM's staff categories — found by sweeping Hulshoff's eid after every
person-side and member-frame lead failed. Every member of every run
checked was pure staff; with the four-in-five non-player gate, 6,653
staff carry an employer on Day One (Hulshoff to Liverpool in the test).

**Aged saves work too** (5 August 2026): staff turnover shuffles the lists
(like squad lists) and the growing record pushes them deeper — 18KB past
the head in a 2031 career against 6KB on day one — so the scanner takes
any in-range count-prefixed run inside the club span (cap 64KB) and the
four-in-five gate carries acceptance. Verified name-by-name against the
running career's own staff screen: Truck, Nicholas and Evans all read
Port Talbot. 6,641 employed staff on that save. A hard-won method note:
an earlier session concluded a shuffled list sat in the *wrong* club's
span — that was an artifact of cross-referencing dumps taken minutes
apart from a save FM was actively rewriting. Snapshot the file before
comparing offsets.

**The two unnamed sheet slots (45, 46) stay unnamed — one hypothesis
killed** (5 August 2026). An in-game staff report for Mats Hummels (slot
45 = 14 against Authority "Good", 46 = 15 against Determination "Very
Good") suggested Authority/Determination, but Nikolić's banked editor
sheet kills 45 = Authority: his editor Authority is 17 and his slot 45
reads 13. Slot 46 tracks Determination on all of Nikolić (13 vs 13),
Fradley (14 vs 13) and Hummels — but index 13 already carries
Determination, so 46 would be a duplicate copy; not named on one
person's word-bands. Side-finding worth keeping: **index 13
(Determination) stores ×5 even inside the tendency half, even on day
one** — Nikolić 68 ≈ 13.6×5 against editor 13, Fradley 63 ≈ 12.6×5
against 13, exactly the "CA-weighted attributes store fine-grained"
rule; the sheet display shows it raw, so it reads as an off-scale
tendency when it is really Determination×5.

**Coarse roles are SOLVED** (5 August 2026): the three lists are a
back-to-back department triple in fixed order — medical, coaching,
recruitment — verified member-by-member against the 2031 career's staff
screen, and the roster seat marks the manager. `Person::staff_role`
carries it; the UI shows it in the Position/Role column and filters on
it under the Staff kind. Not covered: the fine-grained job (a physio vs
the head physio, a coach vs a set-piece coach) — FM stores the words
somewhere still unfound — and the director of football and chairperson,
who appear in none of the three lists. (The `01 [tag] [uid][uid]` rows
nearby resolve to no person — team-entity references, not roles.)

Dead ends recorded so nobody re-treads them: no employer id within ±768
bytes of a staff record (`staffclub`); `manager_manager.dat` holds one odd
record, not a registry; `hall_of_fame.dat` is an award registry (Klopp
reads Liverpool there while unemployed); the roster entry's list is player
registrations, and its other header words resolve to implausible people.

**Staff wages remain unfound** — the eid-anchored wage row players carry
is absent or differently shaped before staff records.

## 4. Contracts — wage and expiry solved, the rest of the block open

Wage and contract expiry parse (see `SAVE_FORMAT.md` §6e): the block sits
just before the person's record prefix, wage anchored on the person's own
entity id, expiry after an 8×FF run. Verified exactly against FM Scout's
Haaland (£450K to 30/6/2034) and Musiala's in-game report in the 2035 save
(£392,499 inside the scouted band, to 30/6/2037).

**Transfer value is not stored — editor-confirmed 3 Aug 2026.** The pre-game
editor exposes a per-player DB "Transfer Value" (Haaland £120,000,000), and
a day-one save contains that figure nowhere near his record in any /10^k
u32 encoding (`valuefind` example; the frame's only 120M hits are
competition prize money before the person table). FM recomputes value at
runtime from ability, age, contract and reputation, so any displayed value
would be an invented number. Original evidence below stands.

The asking-price range on a player's header
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

## 10. Stub people — parsed as presences, names undecoded

Squad members without person records (generated non-contract signings,
`SAVE_FORMAT.md` §6d-bis) now parse as stubs and show as undecoded rows.
What remains is decoding the stub body so the rows can say who they are:

1. **The name is probably not stored.** The u32 at +29 was the only
   name-id candidate, and a survey of all 744 squad-referenced stubs
   (`stubfields` example) killed it: its "100% pool hit rate" is an
   artifact of small ids resolving in every dense pool, the renderings
   are flavour-garbage in all three pools (Malaysian forenames for
   Presteigne and Glynneath fillers), and the id repeats across stubs of
   the same club (LAFC 132 twice, HB 621 twice) — it is a club-correlated
   entity reference, birth city being the natural candidate. With only
   ~33 bytes of body and no other id-shaped field, the likely truth is
   that FM generates grey identities on demand and the save never stores
   them — in which case "Unnamed" is correct-by-design, not a gap. One
   user check settles it: note a filler's in-game name, restart FM,
   reload — if the name changed, it is generated, and this closes.
2. **The age.** The byte at +28 is bimodal across the 744: 17-22 for
   two-thirds (age-shaped — fillers are young) and exactly 100 for the
   rest, so there are two body layouts and the byte cannot be shipped as
   an age until one in-game cross-check (any stub player's squad-screen
   age vs the byte) confirms which layout is which.
3. **The date field reads as creation date**: a year-shaped u16 in the
   body reads 2027 for 696 of 744 stubs in a January-2027 save.

The `squadlists`, `squadheads`, `eidprobe`, `greynames` and `stubfields`
examples reproduce the evidence; `clubteams` prints per-club stub counts.

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
