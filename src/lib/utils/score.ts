import type { Player, ScoringProfile } from '$lib/tauri/commands';

/**
 * Scoring a player against a weighting the user defined.
 *
 * FM's own role ratings are computed from weights SI has never published, so
 * Gilet ships no role table — a "Ball-Winning Midfielder" score built from
 * guessed weights would be exactly the invented number the project refuses to
 * show. What the user weights themselves is their own claim, and the UI labels
 * it as such.
 */

/** Weights below this are treated as absent, so a slider dragged to zero
 * removes the attribute rather than dragging the average down. */
const MIN_WEIGHT = 0.01;

/**
 * The weighted mean of the player's attributes, on the same 1-20 scale as the
 * attributes themselves, so a score of 15 reads like an attribute of 15.
 *
 * Null when the player has no attribute block (staff), when the profile
 * weights nothing, or when none of the weighted indices exist on this player —
 * an unscoreable player is not a zero.
 */
export function score(player: Player, profile: ScoringProfile): number | null {
	if (player.attributes.length === 0) return null;

	let total = 0;
	let weight = 0;
	for (const [key, w] of Object.entries(profile.weights)) {
		if (w < MIN_WEIGHT) continue;
		const index = Number(key);
		const value = player.attributes[index];
		if (value === undefined) continue;
		total += value * w;
		weight += w;
	}
	if (weight === 0) return null;
	return Math.round((total / weight) * 10) / 10;
}

/** Scores every player once, keyed by row id. Players who cannot be scored are
 * left out rather than stored as zero. */
export function scoreAll(
	players: readonly Player[],
	profile: ScoringProfile | null
): ReadonlyMap<number, number> {
	const scores = new Map<number, number>();
	if (profile === null) return scores;
	for (const player of players) {
		const value = score(player, profile);
		if (value !== null) scores.set(player.id, value);
	}
	return scores;
}

/** Indices the profile actually weights, in attribute order. */
export function weightedIndices(profile: ScoringProfile): number[] {
	return Object.entries(profile.weights)
		.filter(([, w]) => w >= MIN_WEIGHT)
		.map(([key]) => Number(key))
		.sort((a, b) => a - b);
}

/** Describes a profile by the attributes it leans on hardest, for a subtitle
 * like "Pace, Acceleration, Finishing". */
export function describeProfile(profile: ScoringProfile, names: readonly string[]): string {
	const top = Object.entries(profile.weights)
		.filter(([, w]) => w >= MIN_WEIGHT)
		.sort((a, b) => b[1] - a[1])
		.slice(0, 3)
		.map(([key]) => names[Number(key)] || `#${key}`);
	return top.join(', ');
}
