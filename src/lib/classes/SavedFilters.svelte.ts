import { loadFilters, saveFilters, type SavedFilter } from '$lib/tauri/commands';
import { emptyFilters, type Filters } from '$lib/utils/filter';

/**
 * Named filter presets, so a recurring search — "free-agent strikers under 23
 * with CA over 120" — survives the session.
 *
 * Presets are merged over {@link emptyFilters} on load rather than trusted
 * wholesale: a preset saved by an older build is missing whatever filters have
 * been added since, and reading it back raw would leave those fields
 * undefined and quietly match nothing.
 */
class SavedFilters {
	presets = $state<SavedFilter[]>([]);
	error = $state<string | null>(null);

	async load(): Promise<void> {
		try {
			this.presets = await loadFilters();
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/** The named preset's filters, with any fields it predates filled in. */
	get(name: string): Filters | null {
		const found = this.presets.find((p) => p.name === name);
		if (!found) return null;
		return { ...emptyFilters, ...(found.filters as Partial<Filters>) };
	}

	/** Saves under `name`, replacing a preset of the same name. */
	async save(name: string, filters: Filters): Promise<void> {
		const trimmed = name.trim();
		if (trimmed === '') return;
		const entry: SavedFilter = { name: trimmed, filters: $state.snapshot(filters) };
		const existing = this.presets.findIndex((p) => p.name === trimmed);
		this.presets =
			existing >= 0
				? this.presets.map((p, i) => (i === existing ? entry : p))
				: [...this.presets, entry];
		await this.persist();
	}

	async remove(name: string): Promise<void> {
		this.presets = this.presets.filter((p) => p.name !== name);
		await this.persist();
	}

	private async persist(): Promise<void> {
		try {
			await saveFilters($state.snapshot(this.presets));
			this.error = null;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}
}

export const savedFilters = new SavedFilters();
