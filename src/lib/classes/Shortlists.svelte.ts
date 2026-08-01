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

	get active(): Shortlist | null {
		return this.lists.find((l) => l.name === this.activeName) ?? null;
	}

	/** Names on the active list, for the row checkboxes. */
	get activeMembers(): ReadonlySet<string> {
		return new Set(this.active?.players ?? []);
	}

	async load(): Promise<void> {
		try {
			this.lists = await loadShortlists();
			this.activeName ??= this.lists[0]?.name ?? null;
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

	/** Adds or removes a player on the active list. */
	async toggle(playerName: string): Promise<void> {
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
