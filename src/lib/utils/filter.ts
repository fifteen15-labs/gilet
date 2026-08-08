import type { Club } from '$lib/tauri/commands';
import { SET_PIECES } from '$lib/utils/attributes';
import { type PositionTier } from '$lib/utils/positions';

export type SortKey =
	| 'name'
	| 'age'
	| 'ability'
	| 'potential'
	| 'score'
	| 'headroom'
	| 'reputation';
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
	/** Only staff in this backroom role — the manager seat or a department.
	 * An unknown role fails the filter: the save not saying is not a match. */
	staffRole: 'any' | 'Manager' | 'Coaching' | 'Medical' | 'Recruitment';
	/** Only players comfortable in this position, e.g. "ST". Null for any. */
	position: string | null;
	/** Which rating counts as playing there: `natural` (15+, their own
	 * position) or `accomplished` (10+, can do a job). Governs the position
	 * filter and the "covers N" count alike. Absent on presets saved before
	 * tiers existed — treated as `natural`, which is what they meant. */
	positionTier: PositionTier;
	/** Only members of the in-save shortlist with this name; `''` is FM's
	 * unnamed default list. Null for any. */
	shortlist: string | null;
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
	/** Minimum worldwide reputation, 0-200, from the non-player sheet. World
	 * reputation is the one that decides who will take your call, which is why
	 * it is the one filtered on. Staff only — a player's reputation is not
	 * decoded, so a player fails this bound rather than passing at zero. */
	minReputation: number | null;
	/** Minimum hidden Professionalism, 1-20 — the strongest single driver of a
	 * young player reaching their potential. Null for any. */
	minProfessionalism: number | null;
	/** Minimum hidden Ambition, 1-20 — whether they want the step up. */
	minAmbition: number | null;
};

export const emptyFilters: Filters = {
	query: '',
	maxAge: null,
	minAbility: null,
	maxAbility: null,
	minPotential: null,
	maxPotential: null,
	kind: 'all',
	staffRole: 'any',
	position: null,
	positionTier: 'natural',
	shortlist: null,
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
	minSetPiece: null,
	minReputation: null,
	minProfessionalism: null,
	minAmbition: null
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

/** A min/max pair phrased the way a scout would say it. */
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
	if (filters.kind === 'staff' && (filters.staffRole ?? 'any') !== 'any')
		parts.push(filters.staffRole);
	if (filters.gender === 'men') parts.push('Men');
	if (filters.gender === 'women') parts.push('Women');
	if (filters.contract === 'free') parts.push('Free agents');
	if (filters.contract === 'expiring') parts.push('Expiring contracts');
	if (filters.risk === 'clean') parts.push('No red flags');
	if (filters.risk === 'flagged') parts.push('Red flags');
	if (filters.shortlist !== null)
		parts.push(`on ${filters.shortlist === '' ? '(unnamed)' : filters.shortlist}`);
	if (filters.position !== null)
		parts.push(
			(filters.positionTier ?? 'natural') === 'accomplished'
				? `${filters.position} (10+)`
				: filters.position
		);
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
	if (filters.minPositions !== null)
		parts.push(
			`covers ${filters.minPositions}+${
				(filters.positionTier ?? 'natural') === 'accomplished' ? ' (10+)' : ''
			}`
		);
	if (filters.minReputation !== null) parts.push(`world rep ${filters.minReputation}+`);
	if (filters.minProfessionalism !== null)
		parts.push(`Professionalism ${filters.minProfessionalism}+`);
	if (filters.minAmbition !== null) parts.push(`Ambition ${filters.minAmbition}+`);
	if (filters.setPiece !== null && filters.minSetPiece !== null) {
		const skill = SET_PIECES.find((s) => s.key === filters.setPiece);
		if (skill) parts.push(`${skill.label} ${filters.minSetPiece}+`);
	}
	if (filters.query.trim() !== '') parts.push(`"${filters.query.trim()}"`);
	return parts.length > 0 ? parts.join(' · ') : 'All players';
}

/**
 * Whether the bar is filtering anything at all — what the "save this filter"
 * and "add these results" buttons are gated on.
 *
 * Compared against {@link emptyFilters} field by field rather than by a hand-
 * written list of conditions: the list was one edit behind every new filter,
 * and a filter missing from it left the buttons dead while the table was
 * plainly narrowed.
 */
export function hasAnyFilter(filters: Filters): boolean {
	const defaults = emptyFilters as unknown as Record<string, unknown>;
	const current = filters as unknown as Record<string, unknown>;
	for (const key of Object.keys(defaults)) {
		// The cutoff is written alongside `contract`, never chosen on its own.
		if (key === 'expiryCutoff') continue;
		// A preset saved before a filter existed carries no key for it, which
		// is the default rather than a difference from it.
		const value = current[key] ?? defaults[key];
		if (key === 'query') {
			if (typeof value === 'string' && value.trim() !== '') return true;
			continue;
		}
		if (value !== defaults[key]) return true;
	}
	return false;
}

/** Clubs match on either their full or short name, so "Man City" and
 * "Manchester City" both find the same club. */
export function matchesClub(club: Club, filters: Filters): boolean {
	const query = filters.query.trim();
	if (query === '') return true;
	const needle = normalise(query);
	return normalise(club.name).includes(needle) || normalise(club.short_name).includes(needle);
}
