import type { Club, Player } from '$lib/tauri/commands';

export type SortKey = 'name' | 'age' | 'ability' | 'potential';
export type SortDirection = 'asc' | 'desc';

export type Filters = {
	query: string;
	maxAge: number | null;
	/** Minimum Current Ability. Has no effect until CA is decoded from the save
	 * format — every player's `ability` is null, so applying it would hide
	 * everyone. The UI keeps the control disabled until then. */
	minAbility: number | null;
	/** Minimum Potential Ability. Same caveat as `minAbility`. */
	minPotential: number | null;
	shortlistedOnly: boolean;
};

export const emptyFilters: Filters = {
	query: '',
	maxAge: null,
	minAbility: null,
	minPotential: null,
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
	// An unknown ability is not a low one. Until CA/PA are decoded these are
	// null for everyone, so an ability filter excludes rather than includes.
	if (filters.minAbility !== null) {
		if (player.ability === null || player.ability < filters.minAbility) return false;
	}
	if (filters.minPotential !== null) {
		if (player.potential === null || player.potential < filters.minPotential) return false;
	}
	if (filters.query.trim() === '') return true;
	return normalise(player.name).includes(normalise(filters.query.trim()));
}

/** Names a shortlist after the search that produced it, so a saved list says
 * what it was rather than "Shortlist 3". */
export function describeFilters(filters: Filters): string {
	const parts: string[] = [];
	if (filters.maxAge !== null) parts.push(`Under ${filters.maxAge}`);
	if (filters.minAbility !== null) parts.push(`CA ${filters.minAbility}+`);
	if (filters.minPotential !== null) parts.push(`PA ${filters.minPotential}+`);
	if (filters.query.trim() !== '') parts.push(`"${filters.query.trim()}"`);
	if (filters.shortlistedOnly) parts.push('shortlisted');
	return parts.length > 0 ? parts.join(' · ') : 'All players';
}

/** True when ability data has been decoded for at least one player, which is
 * what the ability filters need to be meaningful. */
export function hasAbilityData(players: readonly Player[]): boolean {
	return players.some((p) => p.ability !== null);
}

/** Clubs match on either their full or short name, so "Man City" and
 * "Manchester City" both find the same club. */
export function matchesClub(club: Club, filters: Filters): boolean {
	const query = filters.query.trim();
	if (query === '') return true;
	const needle = normalise(query);
	return normalise(club.name).includes(needle) || normalise(club.short_name).includes(needle);
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
