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

## 2. Attribute names — 14 of 54 identified

**Problem.** The format does not label its attributes. Fourteen are named (see
`SAVE_FORMAT.md` §6c); the rest show as "Attribute N".

**Method that worked**, for anyone continuing: three independent signals, and an
index is only named when they agree.

- Which well-known players top the index across the database.
- How the mean shifts by the player's strongest position (needs positions, which
  are decoded — see `research/byposition.py`).
- Goalkeepers as a discriminator, which is what finally separated Heading from
  Jumping Reach: keepers average 13.12 at index 39 but 5.39 at index 3, because
  they jump constantly and head almost never.

**Specifically unsolved: Pace against Acceleration (indices 34 and 38).**
Keepers average 9.29 and 9.09; wingers 13.39 and 13.04; age correlation is
+0.158 and +0.142. Every signal tried returns noise. These two attributes are
near-identical in how they distribute across a football database, which may make
them genuinely inseparable without ground truth. **The cheap fix is the in-game
editor**: read Pace and Acceleration for three or four players and the pair
resolves immediately. Same for the remaining 29 outfield attributes.

**A caveat to carry forward.** Marking (5) and Tackling (9) are named on a
directional signal only — index 5 has a centre-back-to-defensive-mid gap of
+1.08 against index 9's +0.28. If that reasoning is wrong, the two are swapped.
Both stay within the defensive group either way, so the damage is bounded, but it
is the least certain label shipped.

**A mistake worth knowing about.** Index 6 was first labelled Heading because
Ronaldo, Haaland and Mitrović top it. Once positions were decoded, centre-backs
averaged 8.5 there against strikers' 12.0 — and centre-backs head the ball
constantly. It is attacking movement, now Off the Ball. Player-topping evidence
alone is not sufficient; always cross-check against position.

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
apart. The in-game editor would settle it in minutes.

---

## 4. Contracts, value and wages — untouched

The person record contains a variable-length run of 16-byte key/value rows
starting around +48 past the name. Each looks like a 6-byte key then a `u16`
value, and the values are far too large to be attributes: 13961, 11004, 3136,
370, 160. These are plausibly wages, transfer values, contract clauses and
squad-status flags.

Nobody has tried to parse them properly. FM Scout's own listing gives Haaland a
£450K weekly wage and a £179M value in FM26, which are concrete numbers to
search for as an entry point.

---

## 5. Nation names — not in the save

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
