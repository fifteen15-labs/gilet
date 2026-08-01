import { loadProfiles, saveProfiles, type ScoringProfile } from '$lib/tauri/commands';

/**
 * Owns the user's scoring profiles — named attribute weightings used to rank
 * players by what they are actually looking for.
 *
 * Nothing here is seeded. Gilet ships no role weights because SI does not
 * publish FM's, and a table of guessed ones would be an invented number
 * wearing a familiar name.
 */
class Profiles {
	list = $state<ScoringProfile[]>([]);
	activeName = $state<string | null>(null);
	error = $state<string | null>(null);

	readonly active: ScoringProfile | null = $derived(
		this.list.find((p) => p.name === this.activeName) ?? null
	);

	async load(): Promise<void> {
		try {
			this.list = await loadProfiles();
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	/** Saves under `name`, replacing a profile of the same name, and makes it
	 * the active one so the score column reflects what was just edited. */
	async save(name: string, weights: Record<string, number>): Promise<void> {
		const trimmed = name.trim();
		if (trimmed === '') return;
		const entry: ScoringProfile = { name: trimmed, weights: { ...weights } };
		const at = this.list.findIndex((p) => p.name === trimmed);
		this.list = at >= 0 ? this.list.map((p, i) => (i === at ? entry : p)) : [...this.list, entry];
		this.activeName = trimmed;
		await this.persist();
	}

	async remove(name: string): Promise<void> {
		this.list = this.list.filter((p) => p.name !== name);
		if (this.activeName === name) this.activeName = null;
		await this.persist();
	}

	private async persist(): Promise<void> {
		try {
			await saveProfiles($state.snapshot(this.list));
			this.error = null;
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
	}
}

export const profiles = new Profiles();
