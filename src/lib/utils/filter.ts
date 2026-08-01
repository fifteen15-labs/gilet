import type { Player } from '$lib/tauri/commands';

export type SortKey = 'name' | 'age' | 'ability' | 'potential';
export type SortDirection = 'asc' | 'desc';

export type Filters = {
	query: string;
	maxAge: number | null;
	shortlistedOnly: boolean;
};

export const emptyFilters: Filters = {
	query: '',
	maxAge: null,
	shortlistedOnly: false
};

/**
 * Case- and accent-insensitive match, so searching "mbappe" finds "Mbappé" and
 * "nergard" finds "Nergård". Player names in the database are full of
 * diacritics and nobody types them.
 */
export function normalise(value: string): string {
	return value
		.normalize('NFD')
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase();
}

export function matches(player: Player, filters: Filters, shortlisted: ReadonlySet<string>): boolean {
	if (filters.shortlistedOnly && !shortlisted.has(player.name)) return false;
	if (filters.maxAge !== null && player.age > filters.maxAge) return false;
	if (filters.query.trim() === '') return true;
	return normalise(player.name).includes(normalise(filters.query.trim()));
}

/**
 * Sorts in place on a copy. Players with no ability value sort last regardless
 * of direction — an unknown is not a low score, and burying them keeps the
 * top of the table meaningful once abilities are decoded.
 */
export function sortPlayers(players: Player[], key: SortKey, direction: SortDirection): Player[] {
	const factor = direction === 'asc' ? 1 : -1;
	return [...players].sort((a, b) => {
		if (key === 'name') return a.name.localeCompare(b.name) * factor;
		if (key === 'age') return (a.age - b.age) * factor;
		const left = a[key];
		const right = b[key];
		if (left === null && right === null) return a.name.localeCompare(b.name);
		if (left === null) return 1;
		if (right === null) return -1;
		return (left - right) * factor;
	});
}
