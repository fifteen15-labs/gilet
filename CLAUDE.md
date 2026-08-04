# Gilet

A macOS scouting tool for Football Manager saves. Genie Scout is Windows-only
and does not run under Wine; this reads the save file directly instead.

## Design philosophy

**Parse the save, never the process.** FM Scouting Tool and fm-explorer both
read the running game's memory with `ReadProcessMemory`. That is Windows-only
and cannot port — `task_for_pid` on macOS needs root or a debugger entitlement.
Gilet reads the `.fm` file, which is the same format on every platform.

**Never invent a number.** The save format is only partly understood. A field
that has not been confirmed against real data is `None`, and the UI shows an
undecoded state. A wrong Current Ability is worse than a missing one, because
the whole point of the tool is trusting the figure.

**Format knowledge lives in `fm-save`.** That crate is pure: bytes in, data out,
no Tauri and no filesystem. It is testable on its own and every claim in it is
backed by a test. `src-tauri` is a thin I/O shell over it.

**User data on disk, readable.** Shortlists are JSON in Football Manager's own
`shortlists` folder, not a private database. Exports are CSV. FM cannot read
them — its own shortlists are encrypted, and `docs/SHORTLIST_FORMAT.md` §5
records why that road is closed on purpose.

## Layout

```
crates/fm-save/   pure save parser — container, string table, person records
src-tauri/        Tauri 2 shell: commands.rs, shortlist.rs
src/              SvelteKit 5 frontend
docs/             SAVE_FORMAT.md — what is decoded and what is not
                  SHORTLIST_FORMAT.md — FM's own .fmf shortlist archive
research/         Python spikes used to derive the format (throwaway, kept for provenance)
```

## Commands

```bash
bun run desktop:dev              # run the app
bun run check                    # svelte-check
cargo test --workspace           # Rust tests
cargo clippy --workspace --all-targets -- -D warnings
cargo run --release --example dump -- <save.fm>   # parse a save on the CLI
```

## Code standards

Rust follows the same rules as `../trove`: workspace clippy with `all` +
`pedantic`, `unwrap_used` / `expect_used` / `indexing_slicing` denied. Parsing
untrusted bytes must never panic — a malformed save is an error value. Test
modules opt back in with a scoped `#[allow]`.

Frontend follows trove's conventions: Svelte 5 runes, shared state in singleton
classes under `src/lib/classes/*.svelte.ts`, pure transforms in
`src/lib/utils/`, and all Tauri calls behind `src/lib/tauri/commands.ts` so the
backend can be stubbed in one place. Components split at 300 lines.

## Current state

Working: container decompression, the sectioned string table (forename,
surname and common-name pools with overlapping id spaces), person records in
both name layouts (inline full name, or zero-length name composed from the
string table — the case that hides van Dijk and ~37,000 others), person and
club entity ids, **squad membership with captains** via the squad table
validated against club `(eid, uid)` pairs, and attributes, positions,
nationality, Current Ability and Potential Ability, verified against real FM
ratings. **All 54 attribute indices are named** — every visible attribute,
all eleven goalkeeping ones, both feet, and the five hidden ones (Dirtiness
41, Consistency 44, Important Matches 47, Injury Proneness 48, Versatility
49, pinned 3 Aug 2026 by Haaland's pre-game-editor sheet: five distinct
values matching his block one-for-one). The visible set was solved by
intersecting five in-game player reports, then finished against published
FM 26.2 database pages (fminside display×5 values — Donnarumma splits the
last four keeper skills; `docs/SAVE_FORMAT.md` §6c). 75 nations are named from their squads. Aged saves parse:
attribute internals are 1-100 (only initialised to display×5) and squad
lists shuffle after years of transfers, both handled and locked in by a
test against a 2035 save. From a 44 MB save: 49,217 people, 18,663 clubs,
1,814 squads, ~1.5 s. Squad resolution is 99.76% of referenced people; every
player row carries their club's short name. **Wages and contract expiry**
parse from the block before each person record (Haaland £450K/30-6-2034
exact vs FM Scout), shown as a table column and exported in CSV. **The
in-game date reads on both format versions** — the 26.0.0 header pair, or on
26.2.0 the main frame's week stamp with its day-of-year masked from the low
nine bits, gated on the header's format-version string (`SAVE_FORMAT.md`
§1c). The club record's pre-`FF FF` byte is per-club **flags**, not a
constant signature — accepting all values when the entity head validates
recovered Tottenham, Chelsea and ~1,050 other clubs plus their squads in a
26.2.0 save. The club head's third nation u32 is **not a repeat**: it is the
country the club sits in, where the other two are the pyramid it plays in
(`SAVE_FORMAT.md` §4). Requiring all three to match dropped the entity head of
every cross-border club — The New Saints, Cardiff, Swansea, Wrexham, Newport
County, Derry City, Berwick Rangers, F.C. Andorra, Wellington Phoenix — and a
club with no eid cannot be referenced by a squad, so their whole first team
showed no club. Fixing it linked 194 more players on a day-one save.
A squad list is accepted on a **third** signal too: each armband slot being
either unset or one of the club's own players. The old pair — ascending order,
or a captain who is a member — both fail on an aged squad at a club with no
captain appointed (Afan Lido: 18 players, order destroyed by transfers, both
slots `FFFFFFFF`), and the whole squad was dropped. Worth **+2,210 linked
players on a 2030 save**, at a precision cost of 0.13pp.

The save is an **archive with a member manifest** — the last frame names
every other frame (`game_db.dat`, `scout_man.dat`, …), giving one-seek access
to any subsystem (`SAVE_FORMAT.md` §1b). That unlocked **in-game shortlist
import**: the human's shortlists parse from `scout_man.dat` (names in the
clear, members by entity id, §6f) and the sidebar lists them for one-click
import — no touching FM's encrypted `.fmf` exports. **Editing works too**,
verified by FM loading a rewritten save: the detail panel adds/removes
players on in-save shortlists, `archive.rs` rebuilds the file around the
changed member, and the first write parks the untouched original at
`<path>.gilet.bak`. Owner's decision of 2 August 2026 (`LEGAL_NOTES.md`):
writes to the user's own saves only, personal use; key extraction and the
`.fmf` path stay closed.

**The app's one workflow is filter → into the game.** Shortlists in Gilet
*are* the save's shortlists; there is no private list of its own. The filter
bar's "Add N to save" writes every result into a chosen in-save shortlist in
a single archive rewrite, and the detail panel toggles one player at a time.
Gilet-only shortlists, the row checkboxes, "shortlisted only" and CSV
import/export were removed once writing worked — they were scaffolding from
before the save could be edited. **Scoring profiles** — user-set attribute
weights, ranking players by the weighted average — are back in full: the
sidebar picks one, `ProfileEditor.svelte` weights any named attribute 0-5, and
the score is a table column. Gilet still ships no role weights, because FM's
own are unpublished and a guessed table would be an invented number wearing a
familiar name. "Free agents" filters on having neither a contract nor a club: a
missing club alone is a parser gap, not unemployment, and £0 at a club is a
youth deal. Covered by integration tests in `src-tauri/tests/journeys.rs` and
`crates/fm-save/tests/real_save.rs` that run against a real save (Liverpool's
captain must be Virgil van Dijk).

**Scouting reads sit in `src/lib/utils/`, all pure.** `flags.ts` turns the
hidden attributes and the personality run into a scout's report — the values
are the save's, only the threshold each one has to cross is Gilet's, so every
chip carries the number it came from and the panel says so. `audit.ts` reads a
club's squad the way a director of football does (age bands, cover by position,
contracts running down, who is flagged), counting only decoded records and
reporting unresolved squad places as unreadable rather than folding them in.
`attributes.ts` holds the by-name indices the UI filters on; `positions.ts` the
team-sheet order, the pitch layout and `coverage()`. The filter bar gained
**room to grow** (PA − CA, sortable), **red-flag** include/exclude, **max
wage**, **positions covered**, **versatility**, **set-piece** skills and a
**Bargains** screener that clears the bar and sets visible filters rather than
hiding a rule. Throughout, an unknown fails a bound instead of passing it at
zero, and "no reading" is never reported as a clean one.

**The 15 position ratings now render.** They were parsed and shipped over IPC
from the start but nothing displayed them — only the derived naturals list
(15+) reached the UI, which flattened a 20 and a 12 into the same chip. The
detail panel draws them as a pitch (`PositionStrip.svelte`, slot order
`POSITION_NAMES` in `ability.rs`), filled at 15+ and outlined at 10+, and
"Covers N+" filters on how many a player is already rated 15+ in — the honest
versatility measure, since the Versatility *attribute* is about how readily
they learn a new role. **The compare board** (`CompareBoard.svelte`) puts up to
four pinned players side by side over every named attribute; the leader mark
inverts on Dirtiness and Injury Proneness and is withheld entirely when values
tie or only one player has a decoded one.

**Staff attributes parse** (3 Aug 2026, `staff.rs`, `OPEN_PROBLEMS.md` §3b).
`Person::staff` carries the sheet, its three reputations and the non-player
CA/PA, all on the scales the editor shows; `staff::attribute_name` names 50 of
the 52 slots and leaves the two the editor itself leaves blank as `None`.
A real-save test asserts 34 values across Nikolić and Fradley against their
editor pages, and the detail panel draws the sheet (`StaffGrid.svelte`) — the
coaching half, the tendency half and the standing line — for anyone who has
one, where a player gets `AttributeGrid` instead. `staff_attribute_names` sends
the 52 labels once rather than on every row, and the two rows the editor itself
leaves blank render as their slot number. The
54-value block is the pre-game editor's 52-item attribute list at **slots
2-53** — `s0` is a small enum and `s1` ≡ 12, a two-byte header. Items 1-26
(the tendency half, Attacking through Width) store the editor's **raw 1-20**;
items 27-52 (coaching and knowledge) store **raw × 5**, with a per-person
day-one drift of 0 or −2. Not ×4, and not the editor's controlled column.
Three errors had to come undone together: **the block belongs to the object at
person eid − 1**, the non-player object, so every earlier fit compared one
person's sheet with the next person's block; the five u16s before it are
**`[home rep, current rep, world rep, CA, PA]`** and hold the database's own
numbers exactly (Fradley 6350/7000/5500/140/150, Nikolić 6250/6500/4500/130/145
against their editor pages), so **reputation is not regenerated at career
start** as previously concluded; and the fields sit at no fixed stride from the
tag byte, because some objects carry a preamble — find them by signature.
Once rounding to nearest is applied the recovery is *exact*: every non-blank
item on both editor sheets reads back byte for byte, reputations included
(Nikolić home 125 / current 130 / world 90, Fradley 127 / 140 / 110). Items
blank in the database hold generated values, which is what made every
blank-row staff sheet look unfittable.

**Staff identities bind**, which is what made the above testable. The cause was
never the out-of-order eids blamed for it but a scanner off-by-one: an object
header ends in zero bytes, so reading the eid one byte early gives `eid << 8`
with the repeated uid still lining up, and that shadow was accepted and
consumed the real block whenever the shifted eid stayed under `MAX_EID` — every
eid below 11,718. A hit is now dropped when the very next offset carries a
header-proven triple, and a second pass takes the leftovers by shape within 512
bytes of the record prefix. Unbound people on a day-one save: 5,530 → 1,095, at
no cost in parse time. `staffmap` anchors on the header rather than an 8×FF run
and finds 10,422 blocks against the old 1,975.

**All eight hidden personality slots are
named** (Adaptability, Ambition, Loyalty, Pressure, Professionalism,
Sportsmanship, Temperament, Controversy — Guardiola's editor sheet matched
his run exactly) and all eight show in the detail panel.

**Stub people parse** (3 Aug 2026): generated non-contract signings — the
"pay to play" players lower-league squads fill up with — are stored as
~33-byte entity stubs, not person records (`stub.rs`). Squad-referenced
stubs now surface as undecoded rows ("Unnamed — non-contract", no age or
attributes) instead of silently missing from their club's squad; age is
`Option` end-to-end so an unknown age fails an age cap rather than passing
it. Stub name/age fields are not yet decoded (`OPEN_PROBLEMS.md`).

Not yet located: the full nation-name table, the last few attribute names,
and the club link for players outside the loaded leagues (their squads are
not materialised in the squad table, so they show no club).
**`docs/OPEN_PROBLEMS.md` is the handoff document** — it records what was
tried for each, what the evidence showed, and where to go next, including
the small residuals of the squad work.

## Design

Dark only. Cold slate base (`--color-void` `#0e1419`) with a blue cast rather
than near-black. Exactly two accents, each with one job: hi-vis orange for what
the user chose (shortlisted, selected, focus), teal for what the data says
(ability, headroom). IBM Plex in three roles — Condensed for labels and column
headers, Sans for UI, Mono with tabular figures for every number.

The signature element is the ability bar: solid fill for current ability, a
lighter extension for the headroom up to potential. Undecoded ability renders
hatched — an instrument with no reading, not a zero.
