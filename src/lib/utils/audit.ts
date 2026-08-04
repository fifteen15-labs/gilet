/**
 * A squad, read the way a director of football reads one: how old it is, where
 * it is thin, whose contract is running out, and who is carrying a red flag.
 *
 * Everything here is counted from records already decoded. Nothing is
 * estimated: a squad member the parser could not resolve is reported as
 * unreadable rather than folded into the totals, because a thin-looking
 * back line caused by a parser gap would send the user shopping for a problem
 * they do not have.
 */
import type { Club, Player } from '$lib/tauri/commands';
import { headroom, riskCount } from '$lib/utils/flags';
import { POSITIONS } from '$lib/utils/positions';

export type AgeBands = {
	/** 21 and under — the ones with development still ahead of them. */
	young: number;
	/** 22 to 28, the years a player is worth what they cost. */
	prime: number;
	/** 29 and over, where resale value falls away. */
	older: number;
	/** Age undecoded — stubs, mostly. Counted, never guessed at. */
	unknown: number;
};

export type PositionCount = { code: string; count: number };

export type SquadAudit = {
	/** Players matched to this club from the loaded people. */
	counted: number;
	/** Squad places the club record claims that no decoded person fills. */
	unreadable: number;
	ages: AgeBands;
	/** Mean age of the players who have one. Null when none do. */
	averageAge: number | null;
	/** Mean room to grow across players with both ability ends decoded. */
	averageHeadroom: number | null;
	/** Contracts ending on or before the cutoff. */
	expiring: number;
	/** Players with no contract date at all — unknown, not expiring. */
	undated: number;
	/** Players carrying at least one red flag. */
	flagged: number;
	/** Headcount per position, team-sheet order. */
	positions: PositionCount[];
	/** Positions with nobody who plays there. */
	gaps: string[];
};

/** The squad as the loaded people describe it. Clubs are matched on short name
 * because that is the link the person records carry; two clubs sharing one
 * short name would pool, which is why the count is shown against the club's
 * own squad size rather than presented as gospel. */
export function squadOf(club: Club, players: readonly Player[]): Player[] {
	return players.filter((p) => p.club === club.short_name);
}

/** Mean of the values that exist, rounded to one place. Null when there are
 * none — an empty average is not zero. */
function mean(values: number[]): number | null {
	if (values.length === 0) return null;
	const total = values.reduce((sum, v) => sum + v, 0);
	return Math.round((total / values.length) * 10) / 10;
}

/**
 * `expiryCutoff` is a YYYY-MM-DD date, normally a year past the save's own;
 * pass null and the expiring count stays at zero rather than comparing against
 * the wall clock, which has nothing to do with the save.
 */
export function auditSquad(
	club: Club,
	players: readonly Player[],
	expiryCutoff: string | null
): SquadAudit {
	const squad = squadOf(club, players);

	const ages: AgeBands = { young: 0, prime: 0, older: 0, unknown: 0 };
	let expiring = 0;
	let undated = 0;
	let flagged = 0;
	const agesKnown: number[] = [];
	const rooms: number[] = [];
	const perPosition = new Map<string, number>(POSITIONS.map((code) => [code, 0]));

	for (const player of squad) {
		if (player.age === null) ages.unknown += 1;
		else {
			agesKnown.push(player.age);
			if (player.age <= 21) ages.young += 1;
			else if (player.age <= 28) ages.prime += 1;
			else ages.older += 1;
		}

		if (player.contract_until === '') undated += 1;
		else if (expiryCutoff !== null && player.contract_until <= expiryCutoff) expiring += 1;

		if (riskCount(player) > 0) flagged += 1;

		const room = headroom(player);
		if (room !== null) rooms.push(room);

		for (const code of player.positions) {
			const seen = perPosition.get(code);
			if (seen !== undefined) perPosition.set(code, seen + 1);
		}
	}

	const positions = POSITIONS.map((code) => ({ code, count: perPosition.get(code) ?? 0 }));

	return {
		counted: squad.length,
		// A club can list more squad places than the parser resolved people for;
		// it cannot list fewer, so this never goes negative in practice, and is
		// clamped in case a squad table ever says otherwise.
		unreadable: Math.max(0, club.squad_size - squad.length),
		ages,
		averageAge: mean(agesKnown),
		averageHeadroom: mean(rooms),
		expiring,
		undated,
		flagged,
		positions,
		gaps: positions.filter((p) => p.count === 0).map((p) => p.code)
	};
}
