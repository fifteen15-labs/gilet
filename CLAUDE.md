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
ratings. **49 of 54 attribute indices are named** — every visible attribute
including all eleven goalkeeping ones and both feet; only the five hidden
attributes remain. Solved by intersecting five in-game player reports, then
finished against published FM 26.2 database pages (fminside display×5
values — Donnarumma splits the last four keeper skills;
`docs/SAVE_FORMAT.md` §6c). 75 nations are named from their squads. Aged saves parse:
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
26.2.0 save.

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

Search, shortlists, saving a filtered search as a shortlist, and CSV
import/export are wired end to end. **Scoring profiles** let the user weight
attributes themselves and rank players by the weighted average; Gilet ships no
role weights, because FM's own are unpublished and a guessed table would be an
invented number wearing a familiar name. Covered by integration tests in
`src-tauri/tests/journeys.rs` and `crates/fm-save/tests/real_save.rs` that run
against a real save (Liverpool's captain must be Virgil van Dijk).

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
