import { profiles } from '$lib/classes/Profiles.svelte';
import { scoreAll } from '$lib/utils/score';
import {
	addPlayersToGameShortlist,
	clearGameShortlist,
	editGameShortlist,
	onParseProgress,
	openSave,
	type Club,
	type GameShortlist,
	type Player,
	type SaveSummary
} from '$lib/tauri/commands';
import {
	emptyFilters,
	matches,
	matchesClub,
	sortPlayers,
	type Filters,
	type SortDirection,
	type SortKey
} from '$lib/utils/filter';
import { positionSlots } from '$lib/utils/positions';

/** Which record type the table is showing. */
export type Tab = 'people' | 'clubs';

/** Rows rendered at once. The full database is ~12,000 people; searching
 * narrows far faster than scrolling, so the table stays cheap and the count
 * tells the user what is being held back. */
const RENDER_LIMIT = 400;

/** How many players fit side by side before the columns stop being readable. */
const COMPARE_LIMIT = 4;

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
	/** How the clubs table is ordered. Strength is the useful one: it is the
	 * closest thing to a league level while competitions are undecoded. Age
	 * and wages sort on the same decoded squad. */
	clubSort = $state<'name' | 'strength' | 'age' | 'wages'>('name');
	/** Record the detail panel is showing, by row id. */
	selectedId = $state<number | null>(null);
	/** Rows pinned for side-by-side comparison, in the order they were added. */
	pinned = $state<number[]>([]);
	/** Whether the compare board has the main area instead of the table. */
	comparing = $state(false);

	get players(): Player[] {
		return this.summary?.players ?? [];
	}

	get clubs(): Club[] {
		return this.summary?.clubs ?? [];
	}

	get loaded(): boolean {
		return this.summary !== null;
	}

	get selectedPlayer(): Player | null {
		if (this.tab !== 'people' || this.selectedId === null) return null;
		return this.players.find((p) => p.id === this.selectedId) ?? null;
	}

	get selectedClub(): Club | null {
		if (this.tab !== 'clubs' || this.selectedId === null) return null;
		return this.clubs.find((c) => c.id === this.selectedId) ?? null;
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
		this.selectedId = null;
		this.comparing = false;
	}

	/** The pinned players, in pin order — the order the columns appear in. */
	readonly compared: Player[] = $derived(
		this.pinned
			.map((id) => this.players.find((p) => p.id === id))
			.filter((p): p is Player => p !== undefined)
	);

	get compareLimit(): number {
		return COMPARE_LIMIT;
	}

	get compareFull(): boolean {
		return this.pinned.length >= COMPARE_LIMIT;
	}

	isPinned(id: number): boolean {
		return this.pinned.includes(id);
	}

	/** Pins or unpins a player for comparison. Silently refuses past the limit
	 * rather than dropping someone the user pinned on purpose. */
	togglePinned(id: number): void {
		if (this.pinned.includes(id)) {
			this.pinned = this.pinned.filter((p) => p !== id);
			if (this.pinned.length === 0) this.comparing = false;
			return;
		}
		if (this.pinned.length >= COMPARE_LIMIT) return;
		this.pinned = [...this.pinned, id];
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
		this.selectedId = null;
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
	 * Everything matching the current filters, sorted. Not capped — the count
	 * shown to the user has to be the true one.
	 *
	 * Derived, not a method: filtering and sorting 49,000 people is the most
	 * expensive thing the app does, and the filter bar, the table header and
	 * the table body all want the same answer. As a method it ran three times
	 * per keystroke and the UI stopped keeping up with typing.
	 */
	/** Every player's score under the active profile, or empty when none is
	 * selected. Computed once per profile change rather than per row. */
	readonly scores: ReadonlyMap<number, number> = $derived(scoreAll(this.players, profiles.active));

	/** The save's own position slot ordering, so "accomplished at DM" can read
	 * the right rating without a second copy of the list in the frontend. */
	readonly positionSlots: ReadonlyMap<string, number> = $derived(
		positionSlots(this.summary?.position_names ?? [])
	);

	/** Members of the shortlist the filter names, by entity id. Empty when no
	 * shortlist is being filtered to. */
	readonly shortlistEids: ReadonlySet<number> = $derived.by(() => {
		// A preset saved before shortlist filtering existed carries no key at
		// all, and undefined is not a shortlist named "".
		const name = this.filters.shortlist ?? null;
		if (name === null) return new Set<number>();
		const list = (this.summary?.game_shortlists ?? []).find((l) => (l.name ?? '') === name);
		return new Set(list?.player_eids ?? []);
	});

	readonly results: Player[] = $derived.by(() => {
		const context = {
			shortlistEids: this.shortlistEids,
			positionSlots: this.positionSlots
		};
		const found = this.players.filter((p) => matches(p, this.filters, this.scores, context));
		return sortPlayers(found, this.sortKey, this.sortDirection, this.scores);
	});

	readonly visibleResults: Player[] = $derived(this.results.slice(0, RENDER_LIMIT));

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
		// FM's in-save shortlists hold players; a staff row filtered in under
		// a staff search must not be written into one.
		const rows = this.results.filter(
			(p) => p.eid !== null && (p.is_player || p.staff === null)
		);
		const eids = rows.map((p) => p.eid ?? 0);
		if (eids.length === 0) return;
		this.error = null;
		this.notice = null;
		try {
			const added = await addPlayersToGameShortlist(this.summary.path, list.name, eids, [
				year,
				month,
				day
			]);
			const have = new Set(list.player_eids);
			const fresh = rows.filter((p) => p.eid !== null && !have.has(p.eid));
			list.players = [...list.players, ...fresh.map((p) => p.name)];
			list.player_eids = [...list.player_eids, ...fresh.map((p) => p.eid ?? 0)];
			const skipped = this.results.length - rows.length;
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
		this.selectedId = null;
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
		this.selectedId = null;
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
