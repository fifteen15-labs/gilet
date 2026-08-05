import type { Player, ScoringProfile } from '$lib/tauri/commands';
import { STAFF_COACHING_FROM } from '$lib/utils/attributes';

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

/** The attribute sheet a profile's indices point into: the player block, or
 * the 52-item non-player sheet for a staff profile. Null when this person
 * does not carry that sheet — a player profile cannot score staff and a
 * staff profile cannot score a sheetless player. */
function valuesFor(player: Player, profile: ScoringProfile): readonly number[] | null {
	if (profile.kind === 'staff') return player.staff?.attributes ?? null;
	return player.attributes.length > 0 ? player.attributes : null;
}

/**
 * The weighted mean of the person's attributes, on the same 1-20 scale as the
 * attributes themselves, so a score of 15 reads like an attribute of 15.
 *
 * Null when the person does not carry the sheet the profile weights, when the
 * profile weights nothing, or when none of the weighted indices exist — an
 * unscoreable person is not a zero.
 */
export function score(player: Player, profile: ScoringProfile): number | null {
	const values = valuesFor(player, profile);
	if (values === null || values.length === 0) return null;
	// A number on an unknown scale must not enter a 1-20 weighted mean.
	const tendenciesDecoded =
		profile.kind !== 'staff' ||
		values.slice(0, STAFF_COACHING_FROM).every((v) => v <= 20);

	let total = 0;
	let weight = 0;
	for (const [key, w] of Object.entries(profile.weights)) {
		if (w < MIN_WEIGHT) continue;
		const index = Number(key);
		if (profile.kind === 'staff' && index < STAFF_COACHING_FROM && !tendenciesDecoded) continue;
		const value = values[index];
		if (value === undefined || value > 20) continue;
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
