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
import { abilityOf, headroom, riskCount } from '$lib/utils/flags';
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

export type PositionCount = {
	code: string;
	count: number;
	/** Best current ability among the squad players who play there. Null when
	 * nobody does, or none of them has a decoded ability — a position with no
	 * reading is thin, not strong. */
	best: number | null;
};

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

/** Whether a person belongs to this club. Matched on entity id when both sides
 * have one — two club entities can share a short name (most often a men's and
 * women's side), which would pool their squads under a name match — falling
 * back to the short name only for the rare club whose record head never
 * validated an id. */
function atClub(club: Club, player: Player): boolean {
	if (club.eid !== null && player.club_eid !== null) return player.club_eid === club.eid;
	return player.club === club.short_name;
}

/** The squad as the loaded people describe it.
 *
 * Staff are bound to their club too (`backroom.rs`), so the squad is the
 * playing side only — a club's coaches are not part of its age profile. And
 * since the team-squad pass, B and youth players carry the club as well, so
 * the audit keys on the first-team flag: a first-team age profile diluted by
 * sixteen-year-old intakes reads younger than the team that plays. */
export function squadOf(club: Club, players: readonly Player[]): Player[] {
	return players.filter((p) => atClub(club, p) && p.is_player && p.first_team);
}

/** The people the save employs at this club who are not players: the manager
 * from the roster seat, and the three department lists. */
export function backroomOf(club: Club, players: readonly Player[]): Player[] {
	return players.filter((p) => atClub(club, p) && !p.is_player);
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
	const bestPosition = new Map<string, number | null>(POSITIONS.map((code) => [code, null]));

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

		const ability = abilityOf(player);
		for (const code of player.positions) {
			const seen = perPosition.get(code);
			if (seen === undefined) continue;
			perPosition.set(code, seen + 1);
			if (ability !== null) {
				const best = bestPosition.get(code) ?? null;
				if (best === null || ability > best) bestPosition.set(code, ability);
			}
		}
	}

	const positions = POSITIONS.map((code) => ({
		code,
		count: perPosition.get(code) ?? 0,
		best: bestPosition.get(code) ?? null
	}));

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

/**
 * The three department lists the club record carries, in the save's own order.
 * The manager sits outside them, in the roster table's own seat.
 */
export const DEPARTMENTS = ['Coaching', 'Medical', 'Recruitment'] as const;

export type Department = (typeof DEPARTMENTS)[number];

export type DepartmentCount = {
	role: Department;
	count: number;
	/** Mean non-player Current Ability across the department, rounded. Null
	 * when nobody in it has one decoded. */
	averageAbility: number | null;
	/** The strongest world reputation in the department — the name that opens
	 * doors. Null when no sheet in it decoded one. */
	topReputation: number | null;
};

export type BackroomAudit = {
	/** Non-players bound to this club, however they were bound. */
	counted: number;
	/** The manager from the roster seat, when the club has one. */
	manager: Player | null;
	/** The director of football from the boardroom run, when the club's
	 * exact shape decoded and the seat is filled. Null is undecoded-or-vacant,
	 * which the UI must say rather than implying no such person exists. */
	directorOfFootball: Player | null;
	/** The board members from the same run, chair among them, undistinguished
	 * — the save does not say which seat is the chair's. */
	board: Player[];
	departments: DepartmentCount[];
	/** Staff at the club the save gives no department for. They are employed —
	 * they are simply not in one of the three lists, so they are reported
	 * separately rather than counted into a department they may not be in. */
	unassigned: number;
	/** Departments with nobody at all. */
	gaps: string[];
	/** Departments with exactly one person. The line is Gilet's, not FM's: one
	 * physio is a department that stops working the week they are ill. */
	thin: string[];
};

/** Mean of the values that exist, to the nearest whole number. */
function meanRounded(values: number[]): number | null {
	if (values.length === 0) return null;
	return Math.round(values.reduce((sum, v) => sum + v, 0) / values.length);
}

/**
 * A club's backroom, read the way the staff screen reads it: who is in each
 * department, how strong they are, and where there is nobody.
 *
 * Only what the save says. Staff bound to the club with no department land in
 * `unassigned` rather than being distributed across the three, and a
 * department whose members all lack a decoded sheet reports no average rather
 * than an average of nothing.
 */
export function auditBackroom(club: Club, players: readonly Player[]): BackroomAudit {
	const staff = backroomOf(club, players);

	const departments = DEPARTMENTS.map((role) => {
		const members = staff.filter((p) => p.staff_role === role);
		return {
			role,
			count: members.length,
			averageAbility: meanRounded(
				members.map(abilityOf).filter((v): v is number => v !== null)
			),
			topReputation: members.reduce<number | null>((best, p) => {
				const rep = p.staff?.worldReputation;
				if (rep === undefined) return best;
				return best === null || rep > best ? rep : best;
			}, null)
		};
	});

	return {
		counted: staff.length,
		manager: staff.find((p) => p.staff_role === 'Manager') ?? null,
		directorOfFootball: staff.find((p) => p.staff_role === 'Director of Football') ?? null,
		board: staff.filter((p) => p.staff_role === 'Board'),
		departments,
		unassigned: staff.filter((p) => p.staff_role === null).length,
		gaps: departments.filter((d) => d.count === 0).map((d) => d.role),
		thin: departments.filter((d) => d.count === 1).map((d) => d.role)
	};
}
