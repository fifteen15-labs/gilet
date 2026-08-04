/**
 * The outfield position codes, back to front, so a list of them reads like a
 * team sheet rather than the save's own slot ordering.
 *
 * The save carries its own 15 slot names in `position_names`; this is the
 * subset the UI offers as a filter and audits a squad against, in the order a
 * manager thinks about them.
 */
export const POSITIONS = [
	'GK',
	'DL',
	'DC',
	'DR',
	'WBL',
	'WBR',
	'DM',
	'ML',
	'MC',
	'MR',
	'AML',
	'AMC',
	'AMR',
	'ST'
] as const;

export type Position = (typeof POSITIONS)[number];

/**
 * A rating of 15 or better is a position the player genuinely plays; 10 or
 * better is one they can be asked to fill. Both lines are FM's own reading and
 * the same `NATURAL` threshold `ability.rs` uses to build the positions list,
 * so the strip and the position filter never disagree about who is a defender.
 */
export const NATURAL = 15;
export const ACCOMPLISHED = 10;

/**
 * Where each of the save's 15 position slots sits on a pitch, as a row and
 * column in a five-wide grid with the attacking end at the top.
 *
 * Keyed by slot index, because that is what `position_ratings` is indexed by.
 * The order is `POSITION_NAMES` in `crates/fm-save/src/ability.rs` —
 * GK, SW, DL, DC, DR, DM, ML, MC, MR, AML, AMC, AMR, ST, WBL, WBR — established
 * from players whose real position is unambiguous. Labels are read from the
 * save's own `position_names` rather than repeated here, so one list stays
 * authoritative.
 */
export const PITCH: { slot: number; row: number; column: number }[] = [
	{ slot: 12, row: 1, column: 3 }, // ST
	{ slot: 9, row: 2, column: 1 }, // AML
	{ slot: 10, row: 2, column: 3 }, // AMC
	{ slot: 11, row: 2, column: 5 }, // AMR
	{ slot: 6, row: 3, column: 1 }, // ML
	{ slot: 7, row: 3, column: 3 }, // MC
	{ slot: 8, row: 3, column: 5 }, // MR
	{ slot: 5, row: 4, column: 3 }, // DM
	{ slot: 13, row: 5, column: 1 }, // WBL
	{ slot: 14, row: 5, column: 5 }, // WBR
	{ slot: 2, row: 6, column: 1 }, // DL
	{ slot: 3, row: 6, column: 3 }, // DC
	{ slot: 4, row: 6, column: 5 }, // DR
	{ slot: 1, row: 7, column: 3 }, // SW
	{ slot: 0, row: 8, column: 3 } // GK
];

/**
 * How many positions the player is rated at or above `threshold` in. The
 * honest versatility measure: the Versatility attribute is about how readily a
 * player *learns* a new role, not how many they can already fill.
 *
 * Null when the player has no position ratings — staff and stubs — so an
 * unknown fails a coverage filter rather than counting as nought.
 */
export function coverage(ratings: readonly number[], threshold = NATURAL): number | null {
	if (ratings.length === 0) return null;
	return ratings.filter((r) => r >= threshold).length;
}
