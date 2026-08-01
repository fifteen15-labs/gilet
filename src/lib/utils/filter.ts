import type { Club, Player } from '$lib/tauri/commands';

export type SortKey = 'name' | 'age' | 'ability' | 'potential';
export type SortDirection = 'asc' | 'desc';

export type Filters = {
	query: string;
	maxAge: number | null;
	/** Minimum Current Ability, 1-200. Excludes staff, who have no ability. */
	minAbility: number | null;
	/** Minimum Potential Ability, 1-200. Excludes staff. */
	minPotential: number | null;
	/** Restrict to players or to staff. Players are the ones with ability data. */
	kind: 'all' | 'players' | 'staff';
	/** Only players comfortable in this position, e.g. "ST". Null for any. */
	position: string | null;
	shortlistedOnly: boolean;
};

export const emptyFilters: Filters = {
	query: '',
	maxAge: null,
	minAbility: null,
	minPotential: null,
	kind: 'all',
	position: null,
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
	if (filters.kind === 'players' && !player.is_player) return false;
	if (filters.kind === 'staff' && player.is_player) return false;
	if (filters.position !== null && !player.positions.includes(filters.position)) return false;
	if (filters.maxAge !== null && player.age > filters.maxAge) return false;
	// Staff have no ability, and an unknown is not a low score — an ability
	// filter therefore excludes them rather than treating them as zero.
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
	if (filters.kind === 'players') parts.push('Players');
	if (filters.kind === 'staff') parts.push('Staff');
	if (filters.position !== null) parts.push(filters.position);
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
