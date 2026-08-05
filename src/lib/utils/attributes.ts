/**
 * The attribute indices the frontend reaches for by name.
 *
 * All 54 are named in `crates/fm-save/src/ability.rs` and the summary carries
 * that list, so anything showing an attribute to the user should read
 * `attribute_names`. These constants are for the handful of places that need a
 * *specific* attribute — a filter on Versatility, a red flag on Injury
 * Proneness — where the index is part of the logic rather than the label.
 *
 * Every one is confirmed: the visible set against five in-game player reports
 * and published FM 26.2 pages, the five hidden ones against Haaland's
 * pre-game-editor sheet (`docs/SAVE_FORMAT.md` §6c).
 */
export const PENALTY_TAKING = 8;
export const CORNERS = 27;
export const LONG_THROWS = 30;
export const FREE_KICK_TAKING = 35;
export const DIRTINESS = 41;
export const CONSISTENCY = 44;
export const IMPORTANT_MATCHES = 47;
export const INJURY_PRONENESS = 48;
export const VERSATILITY = 49;
export const DETERMINATION = 51;

/**
 * Where the non-player sheet turns from tendencies to coaching, which is also
 * where the save changes storage scale: items before it are the editor's raw
 * 1-20, items from it on are the editor's value times five
 * (`docs/SAVE_FORMAT.md` §6g).
 *
 * The half below this line does not survive ageing — an aged career rewrites
 * it onto an internal scale nobody has decoded — so anything reading a staff
 * sheet has to know where the boundary is before it shows a number.
 */
export const STAFF_COACHING_FROM = 26;

/** The dead-ball skills, in the order a set-piece menu lists them. */
export const SET_PIECES = [
	{ key: 'corners', label: 'Corners', index: CORNERS },
	{ key: 'freekicks', label: 'Free kicks', index: FREE_KICK_TAKING },
	{ key: 'penalties', label: 'Penalties', index: PENALTY_TAKING },
	{ key: 'longthrows', label: 'Long throws', index: LONG_THROWS }
] as const;

export type SetPieceKey = (typeof SET_PIECES)[number]['key'];

/** The attribute index for a set-piece key, or null when the key is unknown —
 * a saved preset from another build must not silently filter on index 0. */
export function setPieceIndex(key: string | null): number | null {
	return SET_PIECES.find((s) => s.key === key)?.index ?? null;
}
