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

**User data on disk, readable.** Shortlists are JSON in the app data directory,
not a private database. Exports are CSV.

## Layout

```
crates/fm-save/   pure save parser — container, string table, person records
src-tauri/        Tauri 2 shell: commands.rs, shortlist.rs
src/              SvelteKit 5 frontend
docs/             SAVE_FORMAT.md — what is decoded and what is not
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

Working: container decompression, string table, person records (name, date of
birth, age), club records (name, short name, club ID, nation ID), and
**attributes, positions, nationality, Current Ability and Potential Ability**,
verified against real FM ratings, positions and nationalities. Players and staff are separable — only players carry an attribute
block. From a 44 MB save: 12,397 people (3,999 players), 18,663 clubs, ~450 ms.

Search, shortlists, saving a filtered search as a shortlist, and CSV
import/export are wired end to end, covered by integration tests in
`src-tauri/tests/journeys.rs` that run against a real save.

Not yet located: squad lists (so no club-to-player link), the full nation-name
table, most individual attribute names, and the in-game date on FM 26.2.0
saves. **`docs/OPEN_PROBLEMS.md` is the handoff document** — it records what was
tried for each, what the evidence showed, and where to go next.

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

**User data on disk, readable.** Shortlists are JSON in the app data directory,
not a private database. Exports are CSV.

## Layout

```
crates/fm-save/   pure save parser — container, string table, person records
src-tauri/        Tauri 2 shell: commands.rs, shortlist.rs
src/              SvelteKit 5 frontend
docs/             SAVE_FORMAT.md — what is decoded and what is not
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

Working: container decompression, string table, person records with full name
and date of birth. 12,397 people parse from a 44 MB save in ~450 ms.

Not yet located: **Current and Potential Ability**, positions, clubs,
attributes. `docs/SAVE_FORMAT.md` records the leads, including the structural
test to apply — PA ≥ CA for every player, which is strong enough on ~12k
records to identify the field pair without needing ground truth.

## Design

Dark only. Cold slate base (`--color-void` `#0e1419`) with a blue cast rather
than near-black. Exactly two accents, each with one job: hi-vis orange for what
the user chose (shortlisted, selected, focus), teal for what the data says
(ability, headroom). IBM Plex in three roles — Condensed for labels and column
headers, Sans for UI, Mono with tabular figures for every number.

The signature element is the ability bar: solid fill for current ability, a
lighter extension for the headroom up to potential. Undecoded ability renders
hatched — an instrument with no reading, not a zero.
