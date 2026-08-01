import type { Club, Player } from '$lib/tauri/commands';

export type SortKey = 'name' | 'age' | 'ability' | 'potential';
export type SortDirection = 'asc' | 'desc';

export type Filters = {
	query: string;
	maxAge: number | null;
	/** Minimum Current Ability, 1-200. Excludes staff, who have no ability. */
	minAbility: number | null;
	/** Maximum Current Ability, 1-200. Finds the affordable end of a bracket. */
	maxAbility: number | null;
	/** Minimum Potential Ability, 1-200. Excludes staff. */
	minPotential: number | null;
	/** Maximum Potential Ability, 1-200. Rules out the ones already at the top. */
	maxPotential: number | null;
	/** Restrict to players or to staff. Players are the ones with ability data. */
	kind: 'all' | 'players' | 'staff';
	/** Only players comfortable in this position, e.g. "ST". Null for any. */
	position: string | null;
	/** Only people of this nation, by identifier. Null for any. */
	nationId: number | null;
	/** Restrict by gender. 'all' keeps people whose gender is unknown too. */
	gender: 'all' | 'men' | 'women';
	/** Contract status: free agents (no club), or contracts expiring soon. */
	contract: 'any' | 'free' | 'expiring';
	/** Latest expiry date (YYYY-MM-DD) that still counts as "expiring soon".
	 * Set alongside `contract: 'expiring'`, from the save's own date. */
	expiryCutoff: string | null;
	shortlistedOnly: boolean;
};

export const emptyFilters: Filters = {
	query: '',
	maxAge: null,
	minAbility: null,
	maxAbility: null,
	minPotential: null,
	maxPotential: null,
	kind: 'all',
	position: null,
	nationId: null,
	gender: 'all',
	contract: 'any',
	expiryCutoff: null,
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
	if (filters.nationId !== null && player.nation_id !== filters.nationId) return false;
	// An unknown gender only passes 'all' — showing a woman under a "Men"
	// filter is wrong, and this save simply cannot say.
	if (filters.gender === 'men' && player.female !== false) return false;
	if (filters.gender === 'women' && player.female !== true) return false;
	// Free agents are the unattached; "expiring" needs a real date on or
	// before the cutoff, so unknown contracts never sneak into a bargain hunt.
	if (filters.contract === 'free' && player.club !== '') return false;
	if (filters.contract === 'expiring') {
		if (player.contract_until === '') return false;
		if (filters.expiryCutoff !== null && player.contract_until > filters.expiryCutoff) return false;
	}
	if (filters.maxAge !== null && player.age > filters.maxAge) return false;
	// Staff have no ability, and an unknown is not a low score — an ability
	// filter therefore excludes them rather than treating them as zero. That
	// holds for an upper bound too: an unknown is not "safely under 120".
	if (filters.minAbility !== null || filters.maxAbility !== null) {
		if (player.ability === null) return false;
		if (filters.minAbility !== null && player.ability < filters.minAbility) return false;
		if (filters.maxAbility !== null && player.ability > filters.maxAbility) return false;
	}
	if (filters.minPotential !== null || filters.maxPotential !== null) {
		if (player.potential === null) return false;
		if (filters.minPotential !== null && player.potential < filters.minPotential) return false;
		if (filters.maxPotential !== null && player.potential > filters.maxPotential) return false;
	}
	if (filters.query.trim() === '') return true;
	// The query matches the club too, so "man city" lists City's squad.
	const needle = normalise(filters.query.trim());
	return normalise(player.name).includes(needle) || normalise(player.club).includes(needle);
}

/** Renders a bounded range the way a scout would say it out loud: "CA 120+",
 * "CA up to 140", or "CA 120-140". Null when neither bound is set. */
function describeRange(label: string, min: number | null, max: number | null): string | null {
	if (min !== null && max !== null) return `${label} ${min}-${max}`;
	if (min !== null) return `${label} ${min}+`;
	if (max !== null) return `${label} up to ${max}`;
	return null;
}

/** Names a shortlist after the search that produced it, so a saved list says
 * what it was rather than "Shortlist 3". */
export function describeFilters(filters: Filters): string {
	const parts: string[] = [];
	if (filters.kind === 'players') parts.push('Players');
	if (filters.kind === 'staff') parts.push('Staff');
	if (filters.gender === 'men') parts.push('Men');
	if (filters.gender === 'women') parts.push('Women');
	if (filters.contract === 'free') parts.push('Free agents');
	if (filters.contract === 'expiring') parts.push('Expiring contracts');
	if (filters.position !== null) parts.push(filters.position);
	if (filters.nationId !== null) parts.push(`nation ${filters.nationId}`);
	if (filters.maxAge !== null) parts.push(`Under ${filters.maxAge}`);
	const ca = describeRange('CA', filters.minAbility, filters.maxAbility);
	if (ca !== null) parts.push(ca);
	const pa = describeRange('PA', filters.minPotential, filters.maxPotential);
	if (pa !== null) parts.push(pa);
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
