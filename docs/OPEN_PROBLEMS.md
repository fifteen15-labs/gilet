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

## 1. Squad membership — the big one

**Problem.** A club cannot be linked to its players. This blocks "show me this
club's squad", a club filter, and knowing which team anyone plays for. It is by
far the most valuable thing still missing.

**Four approaches ruled out, with the evidence:**

1. **A club identifier inside the person record.** There isn't one. Manchester
   City's ID (1075) *does* appear in the person region 427 times at a
   suspiciously consistent 54–60 bytes from a record start — which looks like a
   field until you check membership. Haaland and Grealish, both City players,
   have no reference at all within 4,000 bytes; Walker has one at +838; Arsenal's
   Saka has one at +63. Wrong players, wrong distances. These are almost
   certainly favourite-club or academy affiliations.

2. **A rare value shared between a player and their club.** Haaland and Walker
   each share ~44 `u32` values with the Manchester City record window, but every
   single one is a constant appearing hundreds of times across the file (1024,
   1900, 65536 and similar). Filtering to values used 30 times or fewer leaves
   nothing at all.

3. **An identifier array in the club record body.** There are runs of
   plausible-looking IDs — 29 values at club body +5339, 22 at +5456, in the
   14,000–41,000 range, which is squad-sized and encouraging. But the values
   that also appear near City players (35839, 35584) appear near Arsenal's Saka
   too, so they are constants, not references.

4. **Same-club agreement.** The sharpest test: sweep every offset in ±4000 of
   both the attribute block and the person record, looking for one where
   Haaland, Walker and Grealish all hold the *same* value and Saka, Alisson and
   Dias hold different ones. **Zero hits on both anchors.** Club membership is
   not stored anywhere near the player.

**Why it is hard.** Person records carry no unique identifier of their own. They
hold `first_name_id`, `surname_id` and `common_name_id`, and all three point
into the string table and are shared between namesakes — so there is nothing for
a squad list to reference by, and nothing to search for from the club side.

**Where I would go next.** Find the person's real identifier first; the squad
link is unreachable without it. Two ideas:

- The club record body is only partly understood. After the short name there is
  `01`, six bytes, `02`, then a count byte (8 for Man City) and eight `u32`s
  (6610, 6611, 7578, 7579, 8164, 8593, 9265, 15632). Eight is the right size for
  a club's *teams* — first team, under-21s, under-18s, women's — not a squad.
  Following those IDs to whatever they point at is the most promising lead in
  this whole document: a team record is exactly where a player list should live.
- Diff two saves either side of a transfer. Buy a player in-game, save, and diff:
  the bytes that change are the membership link. `crates/fm-save/examples/diff.rs`
  already does name-matched diffing and would need only a smaller window.

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
