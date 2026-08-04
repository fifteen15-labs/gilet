/**
 * The scout's reading of a player: how much room is left to grow, and the
 * traits that decide whether they will ever use it.
 *
 * Every value here is one the save actually stores — the hidden attributes at
 * their confirmed indices (`crates/fm-save/src/ability.rs`, all 54 named) and
 * the eight hidden personality slots. What Gilet adds is the *threshold*: the
 * line above which "Injury Proneness 17" is worth saying out loud. That line is
 * a reading convention, not an FM figure, so the chips always carry the number
 * they came from and the panel says so in words.
 *
 * An unknown is never a flag and never clears one. Staff and stubs carry no
 * attribute block, so they get no verdict at all rather than a clean bill.
 */
import type { Player } from '$lib/tauri/commands';
import {
	CONSISTENCY,
	DETERMINATION,
	DIRTINESS,
	IMPORTANT_MATCHES,
	INJURY_PRONENESS
} from '$lib/utils/attributes';

export type FlagTone = 'risk' | 'strength';

export type Flag = {
	/** Stable key, for `{#each}` and for tests. */
	key: string;
	/** What a scout would write on the report. */
	label: string;
	/** The stored value the label is drawn from, on FM's 1-20 scale. */
	value: number;
	tone: FlagTone;
	/** Why this value earned a mention, shown on hover. */
	note: string;
};

/** One rule: a value, a direction, and the line it has to cross. */
type Rule = {
	key: string;
	label: string;
	tone: FlagTone;
	note: string;
	/** Pulls the value out of a player; null when the save cannot say. */
	read: (player: Player) => number | null;
	/** 'low' fires at or below the limit, 'high' at or above it. */
	direction: 'low' | 'high';
	limit: number;
};

/** Reads a visible attribute, or null when there is no attribute block —
 * staff and stubs, which must not be judged on values they do not have. */
function attribute(index: number): (player: Player) => number | null {
	return (player) => player.attributes[index] ?? null;
}

/**
 * The thresholds. Deliberately conservative: a chip should mean something is
 * genuinely off, not that a player is a point below average. FM's own scale
 * puts 10 at the league's midpoint, so the risk lines sit at 7 and below and
 * the strength lines at 15 and above — the same bands the attribute grid
 * already colours.
 */
const RULES: Rule[] = [
	{
		key: 'injury-prone',
		label: 'Injury prone',
		tone: 'risk',
		note: 'Gets hurt often, and a wonderkid in the treatment room develops slowly.',
		read: attribute(INJURY_PRONENESS),
		direction: 'high',
		limit: 15
	},
	{
		key: 'inconsistent',
		label: 'Inconsistent',
		tone: 'risk',
		note: 'Performs to their ability only some weeks.',
		read: attribute(CONSISTENCY),
		direction: 'low',
		limit: 8
	},
	{
		key: 'big-game',
		label: 'Fades in big games',
		tone: 'risk',
		note: 'Drops below their level when the match matters most.',
		read: attribute(IMPORTANT_MATCHES),
		direction: 'low',
		limit: 8
	},
	{
		key: 'undetermined',
		label: 'Low determination',
		tone: 'risk',
		note: 'Determination drives training gains — low here wastes headroom.',
		read: attribute(DETERMINATION),
		direction: 'low',
		limit: 8
	},
	{
		key: 'dirty',
		label: 'Dirty',
		tone: 'risk',
		note: 'Concedes needless fouls and cards.',
		read: attribute(DIRTINESS),
		direction: 'high',
		limit: 15
	},
	{
		key: 'unprofessional',
		label: 'Unprofessional',
		tone: 'risk',
		note: 'The strongest single brake on a young player fulfilling their potential.',
		read: (p) => p.professionalism,
		direction: 'low',
		limit: 7
	},
	{
		key: 'volatile',
		label: 'Volatile',
		tone: 'risk',
		note: 'Reacts badly to setbacks and to team talks.',
		read: (p) => p.temperament,
		direction: 'low',
		limit: 6
	},
	{
		key: 'controversial',
		label: 'Controversial',
		tone: 'risk',
		note: 'Says things to the press that cost dressing-room morale.',
		read: (p) => p.controversy,
		direction: 'high',
		limit: 15
	},
	{
		key: 'pressure',
		label: 'Struggles under pressure',
		tone: 'risk',
		note: 'Shrinks at a big club or in a promotion run-in.',
		read: (p) => p.pressure,
		direction: 'low',
		limit: 6
	},
	{
		key: 'unambitious',
		label: 'Unambitious',
		tone: 'risk',
		note: 'Content where they are; hard to push and hard to sign upward.',
		read: (p) => p.ambition,
		direction: 'low',
		limit: 6
	},
	{
		key: 'professional',
		label: 'Professional',
		tone: 'strength',
		note: 'Trains properly. The single best predictor of reaching potential.',
		read: (p) => p.professionalism,
		direction: 'high',
		limit: 15
	},
	{
		key: 'driven',
		label: 'Driven',
		tone: 'strength',
		note: 'High determination — keeps going, and keeps improving.',
		read: attribute(DETERMINATION),
		direction: 'high',
		limit: 15
	},
	{
		key: 'ambitious',
		label: 'Ambitious',
		tone: 'strength',
		note: 'Wants the step up, which makes them gettable and drivable.',
		read: (p) => p.ambition,
		direction: 'high',
		limit: 15
	},
	{
		key: 'durable',
		label: 'Durable',
		tone: 'strength',
		note: 'Rarely injured, so the training time actually lands.',
		read: attribute(INJURY_PRONENESS),
		direction: 'low',
		limit: 4
	},
	{
		key: 'reliable',
		label: 'Reliable',
		tone: 'strength',
		note: 'Turns in their level week to week.',
		read: attribute(CONSISTENCY),
		direction: 'high',
		limit: 15
	}
];

/** Every flag a player has earned, risks first — a report starts with the bad
 * news. Empty for anyone the save cannot say anything about. */
export function flagsFor(player: Player): Flag[] {
	const found: Flag[] = [];
	for (const rule of RULES) {
		const value = rule.read(player);
		if (value === null) continue;
		const fires = rule.direction === 'low' ? value <= rule.limit : value >= rule.limit;
		if (!fires) continue;
		found.push({ key: rule.key, label: rule.label, value, tone: rule.tone, note: rule.note });
	}
	return found.sort((a, b) => (a.tone === b.tone ? 0 : a.tone === 'risk' ? -1 : 1));
}

/**
 * Whether this player can be judged at all. Staff carry no attribute block and
 * stubs carry no record, so "no flags" from them means "no reading", not a
 * clean sheet — the difference decides whether a "no red flags" filter may
 * keep them.
 */
export function hasFlagData(player: Player): boolean {
	return player.attributes.length > 0 || player.professionalism !== null;
}

/** How many risk flags, for the table's compact marker. */
export function riskCount(player: Player): number {
	return flagsFor(player).filter((f) => f.tone === 'risk').length;
}

/**
 * The row's Current Ability on the 0-200 scale: a player's own, or the
 * non-player CA from the staff sheet — both read straight from the save and
 * editor-verified. Null when neither is decoded (stubs), which is not a low
 * one. A player-coach reads as a player: the table is a scouting table first.
 */
export function abilityOf(player: Player): number | null {
	return player.ability ?? player.staff?.currentAbility ?? null;
}

/** The row's Potential Ability, resolved the same way as {@link abilityOf}. */
export function potentialOf(player: Player): number | null {
	return player.potential ?? player.staff?.potentialAbility ?? null;
}

/**
 * Room left to grow: potential minus current ability, on the same 1-200 scale,
 * for players and staff alike — a young coach has headroom too. Null when
 * either end is undecoded — a missing potential is not "no headroom".
 */
export function headroom(player: Player): number | null {
	const current = abilityOf(player);
	const potential = potentialOf(player);
	if (current === null || potential === null) return null;
	return potential - current;
}
