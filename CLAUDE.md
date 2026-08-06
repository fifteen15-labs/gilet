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
last four keeper skills; `docs/SAVE_FORMAT.md` §6c). **228 nations are
named**, 150 from their squads and the other 78 from **club records, which
store their names in the clear and use the same numbering** — "Ba FC, Labasa
FC, Lautoka FC" is Fiji outright, where Fijian-sounding players only suggest
it. That pass named the small federations squad-reading could never settle and
corrected one it had got wrong (116 is Saint Vincent, not Cayman; Cayman is
98). Three ids stay numeric because they name no country. Aged saves parse:
attribute internals are 1-100 (only initialised to display×5) and squad
lists shuffle after years of transfers, both handled and locked in by a
test against a 2035 save. From a 44 MB save: 49,217 people, 18,663 clubs,
1,814 squads, ~1.5 s. Squad resolution is 99.76% of referenced people; every
player row carries their club's short name. **Wages and contract expiry**
parse from the block before each person record (Haaland £450K/30-6-2034
exact vs FM Scout), shown as a table column and exported in CSV. **The
in-game date is `game_db.dat`'s week stamp on both format versions**, its
day-of-year masked from the low nine bits (`SAVE_FORMAT.md` §1c). The header
pair it used to prefer is the **real-world time the file was written**: the
26.2.0 careers here stamp 1-3 August 2026, their own file dates, while sitting
in 2026, 2030, 2032 and 2035. It passed as the in-game date because the
reference save was written weeks after the date it sat at, and the week stamp
was gated to 26.2.0-only because the check masked the *largest* frame instead
of `game_db.dat` — on a long career that is the match-history member, the same
trap `main_frame` carries a note about. An eight-year 26.0.0 career showed it
plainly: header October 2025, database July 2033, sixteen-year-old newgens
reported as eight. The club record's pre-`FF FF` byte is per-club **flags**, not a
constant signature — accepting all values when the entity head validates
recovered Tottenham, Chelsea and ~1,050 other clubs plus their squads in a
26.2.0 save. **The `FF FF` behind it is per-club too** (6 Aug 2026), which the
same failure one step along proved: a new Heybridge Swifts career could not
find its own club, because that record reads `10 FF 00`. The scan now anchors
on the **entity head** and treats all three bytes before the name as data,
requiring `10 FF FF` only where no head validates — +592 clubs, +417 squads and
+5,505 linked players on that save, +97 clubs on a day-one one, no club eid
claimed twice. The head test must run *before* the strings: parsing two
length-prefixed strings at every offset of a 285 MB frame costs a minute where
the head check keeps the whole parse at 3.5 s. The club head's third nation u32 is **not a repeat**: it is the
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
familiar name. **Staff scout the same way** (4 Aug 2026): the table's CA/PA
columns and ability bar read the non-player CA/PA for staff rows (same 0-200
scale, editor-verified), the ability bounds and sort follow, and a profile
now has a kind — a *staff* profile weights the 52-item non-player sheet
(grouped as the editor groups it) and scores staff rows instead. Old profile
JSON has no kind and deserialises as player (`ProfileKind`, serde default).
FM's in-save shortlists hold players, so staff rows are skipped by "Add N to
save" (said in the notice) and the detail panel offers a staff person no add
button. Two honesty rules found in the first real staff-scouting session
(Port Talbot, 2030): FM's non-player CA is **role-weighted**, so an elite
physio legitimately outranks Klopp on raw CA — ranking coaches needs a staff
profile, not the CA column — and **the tendency half of the sheet does not
survive ageing** (Klopp reads 98 where the editor caps at 20; undecoded
internal scale, `SAVE_FORMAT.md` §6g), so the panel shows no tendency
numbers on such a sheet and staff scores skip those slots. **Managers carry their club** (4 Aug
2026, `backroom.rs`): the per-club roster table's manager slot — doubled
club uid, manager eid two u32s past the FF run, FFFF when vacant — binds
1,646 managers on a day-one save and survives ageing (Iraola at Liverpool
in 2030, Klopp correctly unemployed; `SAVE_FORMAT.md` §6h). **The rest of
the backroom binds too** (5 Aug 2026): count-prefixed staff lists in the
club record's body — ascending and shallow on day one, shuffled and up to
18KB deep on aged saves — gated four-in-five on members resolving to
non-players. 6,653 employed staff on Day One (Hulshoff to Liverpool in
the test), 6,641 on a 2031 career, verified against that career's own
staff screen (Truck, Nicholas, Evans → Port Talbot). Method scar: never
cross-reference offsets from two dumps of a save FM is actively
rewriting — snapshot first. **The lists are a department triple** in
fixed order — medical, coaching, recruitment, verified member-by-member
against a running career's staff screen — so `Person::staff_role` names
each employed staff member's department (or Manager, from the roster
seat), the table's Position/Role column shows it, and the Staff kind
gains a role filter. Fine-grained job words (head physio vs physio),
the DOF and the chairperson stay undecoded; staff wages too
(`OPEN_PROBLEMS.md` §3c).
Stubs sort last under the name key: an empty name must not lead the
alphabet. "Free agents" filters on having neither a contract nor a club: a
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
tie or only one player has a decoded one. **It reads staff sheets too** (5 Aug
2026): a board of non-players compares the 52-item sheet in the editor's two
halves, plus CA/PA and the world and current reputations, and withholds the
tendency half when any sheet on the board has aged off the 1-20 scale. A board
holding both kinds falls back to the figures both carry and says so — the two
sheets do not line up index for index, and laying one's labels over the
other's numbers would compare nothing (`utils/compare.ts`).

**Everything already decoded is now reachable from the bar** (5 Aug 2026).
Seven fills, all from data the parser had and only the panel showed:
**world reputation** is a sortable Staff-kind column and a minimum bound (the
reputation that decides who takes your call; home and current stay in the
panel); **an in-save shortlist is a filter** — the row now ships
`player_eids` beside the names, so "only this list" keys on entity ids rather
than names that collide, and the sidebar's per-list Filter button turns a list
built in FM into a search that can be sorted, scored and compared;
**Professionalism and Ambition minimums** screen youth-development targets off
the hidden run; **position ratings have tiers** — natural 15+ or can-cover
10+, one control governing both the position select and the Covers count,
reading the slot ordering out of the save's own `position_names`; the **club
table carries average age and a weekly wage bill**, each computed over the
squad players that decoded and each saying on hover how many that was, since a
partial sum presented as a club's outgoings is an invented number; and
**`auditBackroom`** reads a club's staff the way its staff screen does —
manager seat, the three departments with mean non-player CA and best world
reputation, and the empty or one-deep ones called out with the line named as
Gilet's. The same work found `squadOf` folding a club's coaches into its
squad — staff carry a club since the backroom binding, so a squad audit was
averaging the ages of physios — and a journey test that had been asserting
"people at Man City" is squad-sized (148 on a day-one save, and correct). The
filter bar split to stay inside the 300-line rule — `FilterWho` is row one,
and row two is `FilterBounds`, `FilterTraits` and `FilterActions` — and
`hasAnyFilter` now compares the
bar against `emptyFilters` field by field rather than through a hand-written
list that was one edit behind every new filter. Tests run against
`Career.fm`, or whatever `GILET_TEST_SAVE` names.

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

**The staff scan lost 40% of the sheets to a required header** (fixed 4 Aug
2026, chasing "staff profiles show nothing"). A sheet sits in the tail of the
*previous* person's record behind that person's identity triple — the same
blocks-ahead arrangement player attributes use — and plenty of identities are
written headerless (three zero bytes, no `[type] 40`). `scan_staff` demanded
the header, so every sheet behind a headerless identity vanished: Arne Slot
(CA 165 behind Verberne's bare triple), Arteta, ~7,400 more on Day One —
10,800 → 18,245 sheets bound once the anchor accepts either the header or the
zero bytes. The same headerless shape also dodged the shadow-hit test, which
leaned on the header: Verberne himself was bound to `eid << 8` (526592 for
2057), so `scan_triples` now also drops a hit whose eid and uid both end in a
zero byte when the next offset reads them shifted back. Slot's sheet and
Verberne's true ids are locked in `real_save.rs`.

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

**Compact people parse** (4 Aug 2026): aged saves fold people who leave the
loaded game world — retired, or playing beyond the simulated leagues — down
to a 30-byte entry (`10 00 [forename id][surname id] 01` + entity object)
embedded in the person table with no record prefix, which is why Kylian
Mbappé "went missing" from the 2035 save while day-one saves held him (976
entries there, 0 on day one; `SAVE_FORMAT.md` §6d-ter). `scan_compact` reads
them with the full-record acceptance test (both name ids must resolve, uid
doubled) and they join `Save::people` after every offset-based pass, marked
`Person::compact`, name and identity real and every other field `None` —
`date_of_birth` and `nation_id` went `Option` across the crate to keep that
honest. **The UI does not show them** — a row with no age, club, attributes
or contract answers no scouting question and clogs the table (owner's call,
4 Aug 2026); `commands.rs` filters `Person::compact` out of the row set, so
they exist only in `Save::people` and the format docs.

**A nation identifier past the end of the table means the bytes were not a
person.** Three resolving string ids and a plausible date of birth is a weak
enough test that a 350 MB frame invents about a thousand people per save —
mashup names, no club, no entity id, ages of 9 and 109. FM's highest real
identifier is 249, so a record whose nation reads above 512 is refused. A
day-one save loses 1,043 records, none with a club, and five ability blocks go
back to the real players they belonged to; an eight-year career loses 796 and
its implausible ages fall from 690 to one. The survivor stays: a tighter bound
would start costing real people, and the genuinely old are real — Étienne
Davignon, born 1932, is Anderlecht's honorary chairman.

Not yet located: the last few attribute names, and the club link for players
outside the loaded leagues (their squads are not materialised in the squad
table, so they show no club).
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
