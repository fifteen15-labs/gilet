import type { Club, Player } from '$lib/tauri/commands';
import { SET_PIECES, setPieceIndex, VERSATILITY } from '$lib/utils/attributes';
import { abilityOf, hasFlagData, headroom, potentialOf, riskCount } from '$lib/utils/flags';
import { coverage } from '$lib/utils/positions';

export type SortKey = 'name' | 'age' | 'ability' | 'potential' | 'score' | 'headroom';
export type SortDirection = 'asc' | 'desc';

export type Filters = {
	query: string;
	maxAge: number | null;
	/** Minimum Current Ability, 1-200 — a player's own, or the non-player CA
	 * for staff, so "CA 150+" finds elite coaches under the Staff kind too.
	 * Excludes anyone with neither decoded. */
	minAbility: number | null;
	/** Maximum Current Ability, 1-200. Finds the affordable end of a bracket. */
	maxAbility: number | null;
	/** Minimum Potential Ability, 1-200, player or staff. */
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
	/** Contract status: free agents (no contract), or contracts expiring soon. */
	contract: 'any' | 'free' | 'expiring';
	/** Minimum score under the active scoring profile, on the 1-20 attribute
	 * scale. Null for any; ignored when no profile is active. */
	minScore: number | null;
	/** Latest expiry date (YYYY-MM-DD) that still counts as "expiring soon".
	 * Set alongside `contract: 'expiring'`, from the save's own date. */
	expiryCutoff: string | null;
	/** Minimum room to grow — Potential minus Current Ability, on the 1-200
	 * scale. The development screener's one number. Null for any. */
	minHeadroom: number | null;
	/** Filter on the scout's red flags: 'clean' keeps only players the save can
	 * vouch for, 'flagged' keeps only the ones carrying at least one risk. */
	risk: 'any' | 'clean' | 'flagged';
	/** Highest weekly wage, in the save's display currency. The bargain
	 * board's lever. Null for any. */
	maxWage: number | null;
	/** Minimum Versatility (attribute 49) — how readily the player *learns* a
	 * new role. Null for any. */
	minVersatility: number | null;
	/** Minimum number of positions the player is already rated 15+ in. The
	 * measure that answers "can they cover there on Saturday". Null for any. */
	minPositions: number | null;
	/** Which dead-ball skill to filter on, by {@link SetPieceKey}. Null for
	 * none. */
	setPiece: string | null;
	/** Minimum value for the chosen set-piece skill, 1-20. */
	minSetPiece: number | null;
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
	minScore: null,
	expiryCutoff: null,
	minHeadroom: null,
	risk: 'any',
	maxWage: null,
	minVersatility: null,
	minPositions: null,
	setPiece: null,
	minSetPiece: null
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

export function matches(
	player: Player,
	filters: Filters,
	scores: ReadonlyMap<number, number> = new Map()
): boolean {
	// An unscoreable player is not a low score, so a minimum excludes them
	// rather than treating them as zero — the same rule the ability bounds use.
	if (filters.minScore !== null) {
		const value = scores.get(player.id);
		if (value === undefined || value < filters.minScore) return false;
	}
	if (filters.kind === 'players' && !player.is_player) return false;
	if (filters.kind === 'staff' && player.is_player) return false;
	if (filters.position !== null && !player.positions.includes(filters.position)) return false;
	if (filters.nationId !== null && player.nation_id !== filters.nationId) return false;
	// An unknown gender only passes 'all' — showing a woman under a "Men"
	// filter is wrong, and this save simply cannot say.
	if (filters.gender === 'men' && player.female !== false) return false;
	if (filters.gender === 'women' && player.female !== true) return false;
	// A free agent has neither a contract nor a club. One alone is not
	// enough: contracted players at clubs outside the loaded leagues have no
	// club link yet (OPEN_PROBLEMS §1), and a £0 wage at a club is a youth or
	// amateur deal, not unemployment. An expiry date is contract evidence on
	// its own — some contract layouts parse the date but not the wage, and
	// those players are employed, not free. "Expiring" needs a real date on
	// or before the cutoff, so unknown contracts never sneak into a bargain
	// hunt.
	if (
		filters.contract === 'free' &&
		(player.wage !== null || player.club !== '' || player.contract_until !== '')
	)
		return false;
	if (filters.contract === 'expiring') {
		if (player.contract_until === '') return false;
		if (filters.expiryCutoff !== null && player.contract_until > filters.expiryCutoff) return false;
	}
	// An unknown age is not a young one: stubs (undecoded non-contract
	// fillers) have no birth date, and an age cap must not sweep them in.
	if (filters.maxAge !== null && (player.age === null || player.age > filters.maxAge))
		return false;
	// Ability bounds read whichever ability the row carries — a player's own,
	// or the non-player CA/PA from the staff sheet. An unknown is not a low
	// score, so a row with neither fails the bound rather than passing at
	// zero; that holds for an upper bound too: an unknown is not "safely
	// under 120".
	if (filters.minAbility !== null || filters.maxAbility !== null) {
		const current = abilityOf(player);
		if (current === null) return false;
		if (filters.minAbility !== null && current < filters.minAbility) return false;
		if (filters.maxAbility !== null && current > filters.maxAbility) return false;
	}
	if (filters.minPotential !== null || filters.maxPotential !== null) {
		const potential = potentialOf(player);
		if (potential === null) return false;
		if (filters.minPotential !== null && potential < filters.minPotential) return false;
		if (filters.maxPotential !== null && potential > filters.maxPotential) return false;
	}
	// An unreadable wage is not a cheap one. Players out of contract have no
	// wage to cap, and so does every contract layout the parser has not cracked;
	// neither belongs in a budget search. The "Free agents" toggle is how you
	// ask for the ones costing nothing.
	if (filters.maxWage !== null && (player.wage === null || player.wage > filters.maxWage))
		return false;
	if (filters.minVersatility !== null) {
		const value = player.attributes[VERSATILITY];
		if (value === undefined || value < filters.minVersatility) return false;
	}
	// Position ratings are absent for staff and stubs, and no ratings is not
	// nought positions covered.
	if (filters.minPositions !== null) {
		const covered = coverage(player.position_ratings);
		if (covered === null || covered < filters.minPositions) return false;
	}
	// Both halves have to be set for the filter to mean anything: a skill with
	// no minimum is not a request, and a minimum with no skill has nothing to
	// apply to.
	if (filters.setPiece !== null && filters.minSetPiece !== null) {
		const index = setPieceIndex(filters.setPiece);
		if (index === null) return false;
		const value = player.attributes[index];
		if (value === undefined || value < filters.minSetPiece) return false;
	}
	// Headroom needs both ends decoded. A missing potential is not a player
	// with nothing left to give, so an unknown fails the bound rather than
	// passing it at zero.
	if (filters.minHeadroom !== null) {
		const room = headroom(player);
		if (room === null || room < filters.minHeadroom) return false;
	}
	// Flags are only computed when this filter is on: `matches` runs over every
	// person in the save on each keystroke, and the rule set is the most
	// expensive thing in it.
	//
	// "Clean" means the save vouched for them, not that it stayed silent —
	// staff and stubs carry no attribute block, and no reading is not a good
	// one. "Flagged" needs a real risk, so those same records fail both.
	if (filters.risk !== 'any') {
		if (!hasFlagData(player)) return false;
		const risks = riskCount(player);
		if (filters.risk === 'clean' && risks > 0) return false;
		if (filters.risk === 'flagged' && risks === 0) return false;
	}
	if (filters.query.trim() === '') return true;
	// The query matches the club too, so "man city" lists City's squad.
	const needle = normalise(filters.query.trim());
	return normalise(player.name).includes(needle) || normalise(player.club).includes(needle);
}

export type NationOption = { id: number; name: string };

/**
 * Distinct nations present in the loaded save, named ones alphabetical and the
 * undecoded tail as raw identifiers at the bottom — an unnamed nation still
 * groups and filters correctly, it just cannot say which flag it is.
 */
export function nationsIn(players: readonly Player[]): NationOption[] {
	const seen = new Map<number, string>();
	for (const p of players) {
		// Stubs and compact entries carry no nation at all — nothing to list.
		if (p.nation_id === null) continue;
		if (!seen.has(p.nation_id)) seen.set(p.nation_id, p.nation);
	}
	const named: NationOption[] = [];
	const unnamed: NationOption[] = [];
	for (const [id, name] of seen) {
		if (name === '') unnamed.push({ id, name: `#${id}` });
		else named.push({ id, name });
	}
	named.sort((a, b) => a.name.localeCompare(b.name));
	unnamed.sort((a, b) => a.id - b.id);
	return [...named, ...unnamed];
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
export function describeFilters(filters: Filters, nationName?: string): string {
	const parts: string[] = [];
	if (filters.kind === 'players') parts.push('Players');
	if (filters.kind === 'staff') parts.push('Staff');
	if (filters.gender === 'men') parts.push('Men');
	if (filters.gender === 'women') parts.push('Women');
	if (filters.contract === 'free') parts.push('Free agents');
	if (filters.contract === 'expiring') parts.push('Expiring contracts');
	if (filters.risk === 'clean') parts.push('No red flags');
	if (filters.risk === 'flagged') parts.push('Red flags');
	if (filters.position !== null) parts.push(filters.position);
	if (filters.nationId !== null) parts.push(nationName ?? `nation ${filters.nationId}`);
	if (filters.minScore !== null) parts.push(`score ${filters.minScore}+`);
	if (filters.maxAge !== null) parts.push(`Under ${filters.maxAge}`);
	const ca = describeRange('CA', filters.minAbility, filters.maxAbility);
	if (ca !== null) parts.push(ca);
	const pa = describeRange('PA', filters.minPotential, filters.maxPotential);
	if (pa !== null) parts.push(pa);
	if (filters.minHeadroom !== null) parts.push(`+${filters.minHeadroom} to grow`);
	if (filters.maxWage !== null) parts.push(`under £${filters.maxWage.toLocaleString()}/w`);
	if (filters.minVersatility !== null) parts.push(`Versatility ${filters.minVersatility}+`);
	if (filters.minPositions !== null) parts.push(`covers ${filters.minPositions}+`);
	if (filters.setPiece !== null && filters.minSetPiece !== null) {
		const skill = SET_PIECES.find((s) => s.key === filters.setPiece);
		if (skill) parts.push(`${skill.label} ${filters.minSetPiece}+`);
	}
	if (filters.query.trim() !== '') parts.push(`"${filters.query.trim()}"`);
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
 * One reused collator. `String.prototype.localeCompare` builds its collation
 * options on every call, which is the dominant cost when sorting 49,000 names
 * on each keystroke; a hoisted `Intl.Collator` does that work once.
 */
const byName = new Intl.Collator(undefined, { sensitivity: 'base' });

/** The number a sort key stands for, or null when this person has none of it.
 * Headroom and score are computed rather than stored, so they cannot simply be
 * indexed off the row. */
function value(player: Player, key: SortKey, scores: ReadonlyMap<number, number>): number | null {
	if (key === 'score') return scores.get(player.id) ?? null;
	if (key === 'headroom') return headroom(player);
	if (key === 'ability') return abilityOf(player);
	if (key === 'potential') return potentialOf(player);
	if (key === 'name') return null;
	return player[key];
}

/**
 * Sorts in place on a copy. Players with no ability value sort last regardless
 * of direction — an unknown is not a low score, and burying them keeps the
 * top of the table meaningful once abilities are decoded.
 */
export function sortPlayers(
	players: Player[],
	key: SortKey,
	direction: SortDirection,
	scores: ReadonlyMap<number, number> = new Map()
): Player[] {
	const factor = direction === 'asc' ? 1 : -1;
	return [...players].sort((a, b) => {
		if (key === 'name') {
			// Stubs have no name, and an empty string must not lead the
			// alphabet: the nameless sink to the bottom in both directions,
			// the same way a missing ability does.
			const aEmpty = a.name === '';
			const bEmpty = b.name === '';
			if (aEmpty !== bEmpty) return aEmpty ? 1 : -1;
			return byName.compare(a.name, b.name) * factor;
		}
		const left = value(a, key, scores);
		const right = value(b, key, scores);
		if (left === null && right === null) return byName.compare(a.name, b.name);
		if (left === null) return 1;
		if (right === null) return -1;
		return (left - right) * factor;
	});
}
