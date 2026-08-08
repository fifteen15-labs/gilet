import { profiles } from '$lib/classes/Profiles.svelte';
import {
	addPlayersToGameShortlist,
	clearGameShortlist,
	editGameShortlist,
	onParseProgress,
	openSave,
	searchEids,
	searchPlayers,
	type Club,
	type GameShortlist,
	type MyClub,
	type Player,
	type SaveSummary,
	type Tactic
} from '$lib/tauri/commands';
import {
	emptyFilters,
	matchesClub,
	type Filters,
	type SortDirection,
	type SortKey
} from '$lib/utils/filter';

/** Which record type the table is showing. */
export type Tab = 'people' | 'clubs';

/**
 * One search the user can come back to: a filter set with its sort and
 * selection. The active search's fields live on {@link Scout} directly —
 * everything already reads `scout.filters` — and are copied into this
 * snapshot only when the user switches away.
 */
export type Search = {
	/** Stable key for the tab strip; never reused within a session. */
	id: number;
	filters: Filters;
	sortKey: SortKey;
	sortDirection: SortDirection;
};

/** Rows rendered at once. The full database is ~12,000 people; searching
 * narrows far faster than scrolling, so the table stays cheap and the count
 * tells the user what is being held back. */
const RENDER_LIMIT = 400;

/** How many players fit side by side before the columns stop being readable. */
const COMPARE_LIMIT = 4;

/** How long a keystroke burst may run before a search is sent. Long enough
 * to coalesce typing, short enough that results feel attached to it. */
const SEARCH_DEBOUNCE_MS = 120;

/** Owns the loaded save and how the table is filtered and sorted. */
class Scout {
	summary = $state<SaveSummary | null>(null);
	loading = $state(false);
	/** How far through parsing the backend is, 0 to 1. */
	progress = $state(0);
	/** What the backend is doing right now. */
	progressLabel = $state('');
	error = $state<string | null>(null);
	/** Outcome of the last in-save shortlist write, for the UI to echo. */
	notice = $state<string | null>(null);

	filters = $state<Filters>({ ...emptyFilters });
	sortKey = $state<SortKey>('name');
	sortDirection = $state<SortDirection>('asc');
	tab = $state<Tab>('people');
	/** Every open search. The entry at {@link activeSearch} is a stale
	 * snapshot while its state lives in the fields above; it is refreshed on
	 * every switch away, so the strip must label the active tab from the live
	 * filters, not from here. */
	searches = $state<Search[]>([
		{ id: 1, filters: { ...emptyFilters }, sortKey: 'name', sortDirection: 'asc' }
	]);
	activeSearch = $state(0);
	/** Next search id — ids are per-session, only the strip's keys. */
	private nextSearchId = 2;
	/** How the clubs table is ordered. Strength is the useful one: it is the
	 * closest thing to a league level while competitions are undecoded. Age
	 * and wages sort on the same decoded squad. */
	clubSort = $state<'name' | 'strength' | 'age' | 'wages'>('name');
	/** Record the detail panel is showing, by row id — what highlights the
	 * table row and resolves the selected club. */
	selectedId = $state<number | null>(null);
	/** The selected person's row. Held as the object rather than looked up:
	 * the frontend only has the page of rows the backend returned, and a
	 * selection must survive the filters changing under it. */
	selectedRow = $state<Player | null>(null);
	/** Rows pinned for side-by-side comparison, in the order they were added.
	 * Objects, not ids, for the same reason as {@link selectedRow}. */
	pinned = $state<Player[]>([]);
	/** Whether the compare board has the main area instead of the table. */
	comparing = $state(false);

	/** The current page of results, the true total behind it, and each
	 * visible row's score — all computed by the backend (`search_players`),
	 * which is where the quarter of a million rows live. */
	total = $state(0);
	rows = $state.raw<Player[]>([]);
	scores = $state.raw<ReadonlyMap<number, number>>(new Map());
	/** Bumped for every search so a slow response cannot overwrite a newer
	 * one — replies are only applied when the token still matches. */
	private searchToken = 0;
	private searchTimer: ReturnType<typeof setTimeout> | null = null;

	get clubs(): Club[] {
		return this.summary?.clubs ?? [];
	}

	/** The club the human manages, when the save records one and the human is
	 * employed. Null is the honest "between jobs" or unread state. */
	get myClub(): MyClub | null {
		return this.summary?.my_club ?? null;
	}

	/** The human's active tactic, when one is set. */
	get tactic(): Tactic | null {
		return this.summary?.tactic ?? null;
	}

	/** A manually chosen club, kept when the save records no human (an
	 * unemployed career) or the user is scouting as if for another side. Held
	 * as an entity id, persisted per save path. */
	myClubOverride = $state<number | null>(null);

	/** The club the "my club" tools act on: the manual choice when it resolves,
	 * otherwise the save's own human club. `managerName` is null for a manual
	 * pick — it is a club chosen here, not a job the save records. */
	get activeMyClub(): { eid: number; name: string; shortName: string; managerName: string | null } | null {
		if (this.myClubOverride !== null) {
			const picked = this.clubs.find((c) => c.eid === this.myClubOverride);
			if (picked && picked.eid !== null) {
				const auto = this.myClub;
				return {
					eid: picked.eid,
					name: picked.name,
					shortName: picked.short_name,
					managerName: auto && auto.eid === picked.eid ? auto.managerName : null
				};
			}
		}
		const auto = this.myClub;
		return auto
			? { eid: auto.eid, name: auto.name, shortName: auto.shortName, managerName: auto.managerName }
			: null;
	}

	/** Sets or clears the manual club choice and persists it for this save. */
	setMyClub(eid: number | null): void {
		this.myClubOverride = eid;
		this.persistMyClub();
	}

	private myClubKey(): string | null {
		return this.summary?.path ? `gilet:myclub:${this.summary.path}` : null;
	}

	private persistMyClub(): void {
		const key = this.myClubKey();
		if (key === null || typeof localStorage === 'undefined') return;
		if (this.myClubOverride === null) localStorage.removeItem(key);
		else localStorage.setItem(key, String(this.myClubOverride));
	}

	private loadMyClub(): void {
		const key = this.myClubKey();
		if (key === null || typeof localStorage === 'undefined') {
			this.myClubOverride = null;
			return;
		}
		const stored = localStorage.getItem(key);
		this.myClubOverride = stored === null ? null : Number(stored);
	}

	/** Opens a club in the detail panel by its entity id — the audits render
	 * there. Used by the "my club" shortcuts, which know the eid, not the row
	 * key. Does nothing when the eid names no decoded club. */
	openClubByEid(eid: number): void {
		const club = this.clubs.find((c) => c.eid === eid);
		if (club) {
			this.tab = 'clubs';
			this.comparing = false;
			this.selectedId = club.id;
		}
	}

	/** Lists my club's players in the table, sorted by ability — the "show my
	 * squad" shortcut. Keys on the club's short name, which the search matches
	 * against each row's club. */
	showMySquad(): void {
		const mine = this.activeMyClub;
		if (mine) this.screen({ kind: 'players', query: mine.shortName }, 'ability');
	}

	/** Opens my club's squad audit, where cover-by-position shows where the
	 * squad is thin and each position links to an upgrade search. */
	showWeaknesses(): void {
		const mine = this.activeMyClub;
		if (mine) this.openClubByEid(mine.eid);
	}

	/** Screens the table to players who fit the active tactic — anyone who can
	 * cover (10+) a position the tactic asks for, ranked by ability. The tier
	 * is "can cover" rather than "natural" because a tactic-fit search is about
	 * who could slot in, and the honest natural-only view is one click away on
	 * the tier control the screen leaves visible. */
	showTacticFit(): void {
		const tactic = this.tactic;
		if (!tactic || tactic.positions.length === 0) return;
		const positions = [...new Set(tactic.positions)];
		this.screen(
			{ kind: 'players', tacticPositions: positions, positionTier: 'accomplished' },
			'ability'
		);
	}

	get peopleCount(): number {
		return this.summary?.people_count ?? 0;
	}

	get loaded(): boolean {
		return this.summary !== null;
	}

	get selectedPlayer(): Player | null {
		if (this.tab !== 'people' || this.selectedId === null) return null;
		return this.selectedRow;
	}

	get selectedClub(): Club | null {
		if (this.tab !== 'clubs' || this.selectedId === null) return null;
		return this.clubs.find((c) => c.id === this.selectedId) ?? null;
	}

	/** Opens a person in the detail panel, or closes it with `null`. */
	select(player: Player | null): void {
		this.selectedRow = player;
		this.selectedId = player?.id ?? null;
	}

	matchingClubs(): Club[] {
		const found = this.clubs.filter((c) => matchesClub(c, this.filters));
		// Clubs with nothing decoded for the chosen column sort last in every
		// case: an unknown is not a weak squad, a young one or a cheap one.
		// The youngest squad is the interesting end of the age column, so it
		// ascends where the others descend.
		if (this.clubSort === 'strength') {
			return found.sort((a, b) => (b.average_ability ?? -1) - (a.average_ability ?? -1));
		}
		if (this.clubSort === 'wages') {
			return found.sort((a, b) => (b.wage_bill ?? -1) - (a.wage_bill ?? -1));
		}
		if (this.clubSort === 'age') {
			return found.sort(
				(a, b) => (a.average_age ?? Infinity) - (b.average_age ?? Infinity)
			);
		}
		return found.sort((a, b) => a.name.localeCompare(b.name));
	}

	visibleClubs(): Club[] {
		return this.matchingClubs().slice(0, RENDER_LIMIT);
	}

	show(tab: Tab): void {
		this.tab = tab;
		this.select(null);
		this.comparing = false;
	}

	/** Copies the live filter state into the active search's snapshot. */
	private storeActiveSearch(): void {
		const current = this.searches[this.activeSearch];
		if (!current) return;
		this.searches[this.activeSearch] = {
			id: current.id,
			filters: $state.snapshot(this.filters),
			sortKey: this.sortKey,
			sortDirection: this.sortDirection
		};
	}

	/** Loads a snapshot into the live fields. Merged over {@link emptyFilters}
	 * like a saved preset, for the same reason: a missing field must be the
	 * default, not undefined. Selection does not travel between searches. */
	private restoreSearch(search: Search): void {
		this.filters = { ...emptyFilters, ...search.filters };
		this.sortKey = search.sortKey;
		this.sortDirection = search.sortDirection;
		this.select(null);
	}

	/** Opens a fresh, empty search and switches to it. */
	newSearch(): void {
		this.storeActiveSearch();
		const fresh: Search = {
			id: this.nextSearchId,
			filters: { ...emptyFilters },
			sortKey: 'name',
			sortDirection: 'asc'
		};
		this.nextSearchId += 1;
		this.searches = [...this.searches, fresh];
		this.activeSearch = this.searches.length - 1;
		this.restoreSearch(fresh);
		this.comparing = false;
		this.tab = 'people';
	}

	/** Switches to the search at `index`, keeping the one being left. */
	switchSearch(index: number): void {
		if (index === this.activeSearch) return;
		const target = this.searches[index];
		if (!target) return;
		this.storeActiveSearch();
		this.activeSearch = index;
		this.restoreSearch(target);
		this.comparing = false;
	}

	/** Closes the search at `index`. Closing the last one standing clears it
	 * instead — there is always a search to type into. */
	closeSearch(index: number): void {
		if (!this.searches[index]) return;
		if (this.searches.length === 1) {
			this.searches = [
				{ id: this.nextSearchId, filters: { ...emptyFilters }, sortKey: 'name', sortDirection: 'asc' }
			];
			this.nextSearchId += 1;
			this.activeSearch = 0;
			const only = this.searches[0];
			if (only) this.restoreSearch(only);
			return;
		}
		const wasActive = index === this.activeSearch;
		this.searches = this.searches.filter((_, i) => i !== index);
		if (wasActive) {
			const next = Math.min(index, this.searches.length - 1);
			this.activeSearch = next;
			const target = this.searches[next];
			if (target) this.restoreSearch(target);
			this.comparing = false;
		} else if (index < this.activeSearch) {
			this.activeSearch -= 1;
		}
	}

	/** The pinned players, in pin order — the order the columns appear in. */
	get compared(): Player[] {
		return this.pinned;
	}

	get compareLimit(): number {
		return COMPARE_LIMIT;
	}

	get compareFull(): boolean {
		return this.pinned.length >= COMPARE_LIMIT;
	}

	isPinned(id: number): boolean {
		return this.pinned.some((p) => p.id === id);
	}

	/** Pins or unpins a player for comparison. Silently refuses past the limit
	 * rather than dropping someone the user pinned on purpose. */
	togglePinned(player: Player): void {
		if (this.isPinned(player.id)) {
			this.pinned = this.pinned.filter((p) => p.id !== player.id);
			if (this.pinned.length === 0) this.comparing = false;
			return;
		}
		if (this.pinned.length >= COMPARE_LIMIT) return;
		this.pinned = [...this.pinned, player];
	}

	clearPinned(): void {
		this.pinned = [];
		this.comparing = false;
	}

	/** Jumps to the people table filtered to one club's squad. The query
	 * matches club names, so the club's short name is the whole filter. */
	showSquad(shortName: string): void {
		this.filters = { ...emptyFilters, query: shortName };
		this.tab = 'people';
		this.select(null);
	}

	/**
	 * Filters the table down to one in-save shortlist's members, so a list
	 * built in FM can be sorted, scored and compared like any other search.
	 * `null` clears the shortlist filter and leaves the rest of the bar alone.
	 */
	showShortlist(list: GameShortlist | null): void {
		this.filters = { ...this.filters, shortlist: list === null ? null : (list.name ?? '') };
		this.tab = 'people';
		this.comparing = false;
	}

	/**
	 * Runs the current search in the backend, debounced so a keystroke burst
	 * costs one round trip. Called from an `$effect` in the page that reads
	 * the filters, sort and profile — the reactive graph decides *when*, the
	 * backend does the work, and a stale reply is dropped by token.
	 *
	 * Filtering and sorting a quarter of a million rows used to happen here
	 * in a `$derived`, on the UI thread, per keystroke — which is what made
	 * large searches drag.
	 */
	search(): void {
		if (this.summary === null) return;
		const filters = $state.snapshot(this.filters);
		const sortKey = this.sortKey;
		const sortDirection = this.sortDirection;
		const profile = profiles.active ? $state.snapshot(profiles.active) : null;
		this.searchToken += 1;
		const token = this.searchToken;
		if (this.searchTimer !== null) clearTimeout(this.searchTimer);
		this.searchTimer = setTimeout(() => {
			void searchPlayers(filters, sortKey, sortDirection, profile, RENDER_LIMIT)
				.then((page) => {
					if (token !== this.searchToken) return;
					this.total = page.total;
					this.rows = page.rows;
					const scored = new Map<number, number>();
					page.rows.forEach((row, i) => {
						const value = page.scores[i];
						if (value !== null && value !== undefined) scored.set(row.id, value);
					});
					this.scores = scored;
				})
				.catch((e: unknown) => {
					if (token !== this.searchToken) return;
					this.error = e instanceof Error ? e.message : String(e);
				});
		}, SEARCH_DEBOUNCE_MS);
	}

	get renderLimit(): number {
		return RENDER_LIMIT;
	}

	async open(path: string): Promise<void> {
		this.loading = true;
		this.error = null;
		this.progress = 0;
		this.progressLabel = 'Opening the file';
		// The backend parses on a worker thread and reports each stage, so
		// this stays live rather than freezing on a spinner.
		const stop = await onParseProgress(({ fraction, label }) => {
			this.progress = fraction;
			this.progressLabel = label;
		});
		try {
			this.summary = await openSave(path);
			this.loadMyClub();
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.summary = null;
		} finally {
			stop();
			this.loading = false;
		}
	}

	/**
	 * Whether shortlists inside the save can be edited: the write needs the
	 * save's own date for the entry's date-added field, so an undated save
	 * stays read-only — a wrong date written into the file is an invented
	 * number, and the one rule is never to invent one.
	 */
	get canEditGameShortlists(): boolean {
		return this.summary !== null && this.summary.game_date !== null;
	}

	/**
	 * Adds or removes a player on a shortlist stored in the save file itself.
	 * The backend backs the save up to `<path>.gilet.bak` before its first
	 * write; on success the loaded summary is updated in place rather than
	 * re-parsing 60 MB.
	 */
	async toggleGameShortlist(list: GameShortlist, player: Player): Promise<void> {
		if (!this.summary || !this.summary.game_date || player.eid === null) return;
		const [year, month, day] = this.summary.game_date.split('-').map(Number);
		const has = list.players.includes(player.name);
		this.error = null;
		try {
			await editGameShortlist(this.summary.path, list.name, player.eid, !has, [year, month, day]);
			// Names and ids are kept in step: the sidebar reads the names and
			// the "only this shortlist" filter reads the ids, and a list that
			// disagreed with itself would show one and filter the other.
			list.players = has
				? list.players.filter((n) => n !== player.name)
				: [...list.players, player.name];
			list.player_eids = has
				? list.player_eids.filter((e) => e !== player.eid)
				: [...list.player_eids, player.eid];
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/**
	 * Empties one in-save shortlist. The list itself survives, so the user can
	 * refill it; the pre-Gilet save is still on disk as `.gilet.bak`.
	 */
	async clearGameShortlist(list: GameShortlist): Promise<void> {
		if (!this.summary) return;
		const had = list.players.length;
		this.error = null;
		this.notice = null;
		try {
			await clearGameShortlist(this.summary.path, list.name);
			list.players = [];
			list.player_eids = [];
			this.notice = `Cleared ${had.toLocaleString()} from ${list.name ?? '(unnamed)'} in the save.`;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/**
	 * Writes every current result into one in-save shortlist — the whole point
	 * of the app: filter, then send the results into the game. One save
	 * rewrite regardless of how many players; already-listed ones are skipped
	 * by the backend.
	 */
	async addResultsToGameShortlist(list: GameShortlist): Promise<void> {
		if (!this.summary || !this.summary.game_date) return;
		const [year, month, day] = this.summary.game_date.split('-').map(Number);
		this.error = null;
		this.notice = null;
		try {
			// The full matching set lives in the backend; asking for the ids
			// is how the whole result — not just the visible page — gets
			// written. Staff rows and rows without an entity id are already
			// excluded there: FM's in-save shortlists hold players.
			const profile = profiles.active ? $state.snapshot(profiles.active) : null;
			const hits = await searchEids($state.snapshot(this.filters), profile);
			if (hits.length === 0) return;
			const added = await addPlayersToGameShortlist(
				this.summary.path,
				list.name,
				hits.map((h) => h.eid),
				[year, month, day]
			);
			const have = new Set(list.player_eids);
			const fresh = hits.filter((h) => !have.has(h.eid));
			list.players = [...list.players, ...fresh.map((h) => h.name)];
			list.player_eids = [...list.player_eids, ...fresh.map((h) => h.eid)];
			const skipped = this.total - hits.length;
			this.notice =
				`Added ${added.toLocaleString()} to ${list.name ?? '(unnamed)'} in the save.` +
				(skipped > 0
					? ` ${skipped} skipped — staff, or no entity id: FM's shortlists hold players.`
					: '');
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/**
	 * Re-reads the save from disk, keeping the current filters. For picking up
	 * changes FM made after this copy was loaded — record offsets move when the
	 * game rewrites a save, so the open record is dropped rather than pointed
	 * at whatever now sits at that offset.
	 */
	async reload(): Promise<void> {
		const path = this.summary?.path;
		if (!path) return;
		this.select(null);
		await this.open(path);
	}

	/**
	 * Applies a canned search: the given filters over a cleared bar, ordered by
	 * the column the search is really about.
	 *
	 * Starting from `emptyFilters` rather than patching what is already set is
	 * the point — a screener the user cannot predict is worse than no screener.
	 * Everything it sets stays visible in the filter bar afterwards, so the
	 * button is a shortcut rather than a black box.
	 */
	screen(patch: Partial<Filters>, sortKey: SortKey): void {
		this.filters = { ...emptyFilters, ...patch };
		this.sortKey = sortKey;
		this.sortDirection = 'desc';
		this.select(null);
		this.tab = 'people';
		this.comparing = false;
	}

	/** Clicking a column sorts by it, and clicking the active column reverses. */
	sortBy(key: SortKey): void {
		if (this.sortKey === key) {
			this.sortDirection = this.sortDirection === 'asc' ? 'desc' : 'asc';
			return;
		}
		this.sortKey = key;
		// Names read naturally A-Z; ages and abilities are most useful highest-first.
		this.sortDirection = key === 'name' ? 'asc' : 'desc';
	}

	reset(): void {
		this.filters = { ...emptyFilters };
	}
}

export const scout = new Scout();
