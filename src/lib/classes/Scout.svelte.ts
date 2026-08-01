import { openSave, type Club, type Player, type SaveSummary } from '$lib/tauri/commands';
import {
	emptyFilters,
	matches,
	matchesClub,
	sortPlayers,
	type Filters,
	type SortDirection,
	type SortKey
} from '$lib/utils/filter';

/** Which record type the table is showing. */
export type Tab = 'people' | 'clubs';

/** Rows rendered at once. The full database is ~12,000 people; searching
 * narrows far faster than scrolling, so the table stays cheap and the count
 * tells the user what is being held back. */
const RENDER_LIMIT = 400;

/** Owns the loaded save and how the table is filtered and sorted. */
class Scout {
	summary = $state<SaveSummary | null>(null);
	loading = $state(false);
	error = $state<string | null>(null);

	filters = $state<Filters>({ ...emptyFilters });
	sortKey = $state<SortKey>('name');
	sortDirection = $state<SortDirection>('asc');
	tab = $state<Tab>('people');
	/** Record the detail panel is showing, by row id. */
	selectedId = $state<number | null>(null);

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
		return found.sort((a, b) => a.name.localeCompare(b.name));
	}

	visibleClubs(): Club[] {
		return this.matchingClubs().slice(0, RENDER_LIMIT);
	}

	show(tab: Tab): void {
		this.tab = tab;
		this.selectedId = null;
	}

	/** Everything matching the current filters, sorted. Not capped — the count
	 * shown to the user has to be the true one. */
	matching(shortlisted: ReadonlySet<string>): Player[] {
		const filtered = this.players.filter((p) => matches(p, this.filters, shortlisted));
		return sortPlayers(filtered, this.sortKey, this.sortDirection);
	}

	visible(shortlisted: ReadonlySet<string>): Player[] {
		return this.matching(shortlisted).slice(0, RENDER_LIMIT);
	}

	get renderLimit(): number {
		return RENDER_LIMIT;
	}

	async open(path: string): Promise<void> {
		this.loading = true;
		this.error = null;
		try {
			this.summary = await openSave(path);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.summary = null;
		} finally {
			this.loading = false;
		}
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
