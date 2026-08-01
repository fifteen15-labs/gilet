import { loadShortlists, saveShortlists, type Shortlist } from '$lib/tauri/commands';

/**
 * Owns the user's shortlists and their persistence.
 *
 * Members are stored by player name rather than record offset: offsets are only
 * meaningful inside one save file, and a shortlist should survive loading next
 * season's rollover.
 */
class Shortlists {
	lists = $state<Shortlist[]>([]);
	activeName = $state<string | null>(null);
	error = $state<string | null>(null);

	readonly active: Shortlist | null = $derived(
		this.lists.find((l) => l.name === this.activeName) ?? null
	);

	/**
	 * Names on the active list, for the row checkboxes. Derived rather than a
	 * getter so the set is built once per change: a getter returned a fresh Set
	 * on every read, and the table reads it several times per keystroke.
	 */
	readonly activeMembers: ReadonlySet<string> = $derived(new Set(this.active?.players ?? []));

	/**
	 * Loads from disk without selecting anything. Auto-selecting the first list
	 * meant a large saved list came back active on every launch and pre-ticked
	 * every one of its members across the table, which reads as the app having
	 * chosen them for you.
	 */
	async load(): Promise<void> {
		try {
			this.lists = await loadShortlists();
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	async create(name: string): Promise<void> {
		const trimmed = name.trim();
		if (trimmed === '' || this.lists.some((l) => l.name === trimmed)) return;
		this.lists = [...this.lists, { name: trimmed, players: [] }];
		this.activeName = trimmed;
		await this.persist();
	}

	async remove(name: string): Promise<void> {
		this.lists = this.lists.filter((l) => l.name !== name);
		if (this.activeName === name) this.activeName = this.lists[0]?.name ?? null;
		await this.persist();
	}

	/** Renames a list, refusing a clash rather than merging two lists. */
	async rename(from: string, to: string): Promise<boolean> {
		const trimmed = to.trim();
		if (trimmed === '' || trimmed === from) return false;
		if (this.lists.some((l) => l.name === trimmed)) return false;
		this.lists = this.lists.map((l) => (l.name === from ? { ...l, name: trimmed } : l));
		if (this.activeName === from) this.activeName = trimmed;
		await this.persist();
		return true;
	}

	/** Adds many players to the active list at once — the "keep everything the
	 * filter found" flow. Creates a first list when none exists. */
	async addAll(playerNames: string[]): Promise<void> {
		if (!this.active) {
			await this.create('Shortlist');
		}
		const list = this.active;
		if (!list || playerNames.length === 0) return;
		const merged = [...list.players];
		const have = new Set(merged);
		for (const name of playerNames) {
			if (!have.has(name)) {
				have.add(name);
				merged.push(name);
			}
		}
		this.lists = this.lists.map((l) => (l.name === list.name ? { ...l, players: merged } : l));
		await this.persist();
	}

	/**
	 * Saves a set of players as a new shortlist — the "filter, then keep the
	 * results" flow. Returns the name actually used, which is suffixed if one
	 * already exists so an earlier list is never silently overwritten.
	 */
	async saveAs(name: string, players: string[]): Promise<string | null> {
		const base = name.trim();
		if (base === '' || players.length === 0) return null;

		let unique = base;
		let n = 2;
		while (this.lists.some((l) => l.name === unique)) {
			unique = `${base} (${n})`;
			n += 1;
		}

		this.lists = [...this.lists, { name: unique, players: [...players] }];
		this.activeName = unique;
		await this.persist();
		return unique;
	}

	/** Adds or removes a player on the active list. Ticking a row before any
	 * list exists creates one rather than silently doing nothing. */
	async toggle(playerName: string): Promise<void> {
		if (!this.active) {
			await this.create('Shortlist');
		}
		const list = this.active;
		if (!list) return;
		const players = list.players.includes(playerName)
			? list.players.filter((p) => p !== playerName)
			: [...list.players, playerName];
		this.lists = this.lists.map((l) => (l.name === list.name ? { ...l, players } : l));
		await this.persist();
	}

	private async persist(): Promise<void> {
		try {
			await saveShortlists($state.snapshot(this.lists));
			this.error = null;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}
}

export const shortlists = new Shortlists();
