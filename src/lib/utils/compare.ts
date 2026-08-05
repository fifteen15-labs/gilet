/**
 * What the compare board can honestly put side by side.
 *
 * Players and staff carry different sheets — 54 player attributes against the
 * 52-item non-player one — and the two do not line up index for index. A board
 * therefore reads as one or the other, and a board holding both falls back to
 * the figures they share rather than laying one sheet's labels over the other's
 * numbers.
 */
import type { Player } from '$lib/tauri/commands';
import { STAFF_COACHING_FROM } from '$lib/utils/attributes';

/** Which sheet a board of pinned people can be read against. */
export type BoardMode = 'player' | 'staff' | 'mixed';

export function boardMode(players: readonly Player[]): BoardMode {
	if (players.length === 0) return 'player';
	if (players.every((p) => p.attributes.length > 0)) return 'player';
	// A player-coach carries both; the player block is the one that wins,
	// which is the rule the table already follows.
	if (players.every((p) => p.staff !== null && p.attributes.length === 0)) return 'staff';
	return 'mixed';
}

/**
 * Which columns hold the best value in a row, as a set of column indices.
 *
 * Empty when every value matches or when fewer than two people have one:
 * marking a "winner" among identical numbers, or against someone whose value is
 * simply undecoded, would be reading a result out of nothing.
 */
export function leaders(values: readonly (number | null)[], lowerIsBetter = false): Set<number> {
	const known = values.filter((v): v is number => v !== null);
	if (known.length < 2) return new Set();
	const best = lowerIsBetter ? Math.min(...known) : Math.max(...known);
	const worst = lowerIsBetter ? Math.max(...known) : Math.min(...known);
	if (best === worst) return new Set();
	const found = new Set<number>();
	values.forEach((v, column) => {
		if (v === best) found.add(column);
	});
	return found;
}

/**
 * Whether every staff sheet on the board still reads on the editor's 1-20
 * scale. An aged career rewrites the tendency half onto an internal scale
 * nobody has decoded — one value past 20 proves that half is off the scale for
 * that person, and a row mixing the two scales would compare nothing.
 */
export function staffTendenciesDecoded(players: readonly Player[]): boolean {
	return players.every((p) =>
		(p.staff?.attributes ?? []).slice(0, STAFF_COACHING_FROM).every((v) => v <= 20)
	);
}
