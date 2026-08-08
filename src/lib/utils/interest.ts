/**
 * How open a player is likely to be to joining your club — Gilet's estimate,
 * not FM's own figure.
 *
 * This is a *prediction*, the kind a scouting tool is for, and it is presented
 * as one: every band carries the inputs it was built from ({@link
 * InterestEstimate.reasons}), and the number is never dressed up as the save's
 * own. It leans on what the save does decode — the player's world reputation
 * against the club's, their ambition, loyalty and adaptability, and whether
 * their contract is running down — and deliberately leaves out what it cannot
 * read (agreed playing time, agents, release clauses, league prestige). So it
 * answers "would this move appeal to them?", not "will they sign?".
 *
 * The reputation relationship dominates, as it does in FM: a player rarely
 * drops to a smaller club, and readily joins a bigger one. Ambition sharpens
 * that both ways, loyalty anchors them where they are, a contract running down
 * opens the door, and adaptability makes the upheaval easier to stomach.
 */
import type { Player } from '$lib/tauri/commands';

export type InterestBand = 'Very open' | 'Open' | 'Neutral' | 'Cool' | 'Unlikely';

export type InterestEstimate = {
	band: InterestBand;
	/** Gilet's index, -100 (would refuse) to +100 (would jump at it). */
	score: number;
	/** The inputs that moved the estimate, in plain words, strongest first. */
	reasons: string[];
};

/** Reputation is 0-200 on the player line; the club's is 0-10000. Bringing the
 * player onto the club scale makes the gap a like-for-like difference. */
const REP_TO_CLUB_SCALE = 50;

/** How many reputation points on the club scale move the index one point.
 * A 4000-point gap — roughly a mid-Championship club chasing an elite player,
 * or vice versa — saturates the reputation term. */
const GAP_PER_POINT = 40;

function clamp(value: number, low: number, high: number): number {
	return Math.max(low, Math.min(high, value));
}

/** A hidden 1-20 attribute as a −1…+1 lever, centred on the FM average of 10. */
function lever(value: number | null): number {
	return value === null ? 0 : (value - 10) / 10;
}

function bandOf(score: number): InterestBand {
	if (score >= 50) return 'Very open';
	if (score >= 20) return 'Open';
	if (score > -20) return 'Neutral';
	if (score > -50) return 'Cool';
	return 'Unlikely';
}

/**
 * Estimates a player's openness to joining a club of the given reputation.
 *
 * Returns null when the estimate would rest on a guess: a player whose world
 * reputation did not decode, or a club with no reputation read. A missing input
 * is not a neutral one — better to show nothing than a confident-looking number
 * built on air.
 *
 * @param player the player being scouted
 * @param clubReputation the scouting club's reputation, 0-10000
 * @param expiring whether the player's contract is running down — the page
 *   knows the save's own date and the cutoff that means, this module does not,
 *   so the caller passes its own verdict rather than this one re-deriving it.
 */
export function estimateInterest(
	player: Player,
	clubReputation: number | null,
	expiring: boolean
): InterestEstimate | null {
	const world = player.reputation?.world ?? null;
	if (world === null || clubReputation === null) return null;

	const playerRep = world * REP_TO_CLUB_SCALE;
	const gap = clubReputation - playerRep;
	const reasons: string[] = [];

	// The reputation relationship carries the estimate.
	let score = clamp(gap / GAP_PER_POINT, -100, 100);
	if (gap >= 500) reasons.push('your club is the more reputable side');
	else if (gap <= -500) reasons.push('they are more reputable than your club');
	else reasons.push('you and they are of similar standing');

	// Ambition sharpens the reputation gap in whichever direction it points:
	// the ambitious jump at a bigger club and refuse a smaller one outright.
	const ambition = lever(player.ambition);
	if (player.ambition !== null && Math.abs(ambition) > 0.1) {
		score += ambition * (gap >= 0 ? 18 : -18);
		if (player.ambition >= 15) {
			reasons.push(gap >= 0 ? 'ambitious — wants the step up' : 'ambitious — unlikely to drop down');
		} else if (player.ambition <= 6) {
			reasons.push('unambitious — the size of the move matters less');
		}
	}

	// Loyalty anchors a player where they are, whatever the move on offer.
	if (player.loyalty !== null && player.loyalty >= 15) {
		score -= (player.loyalty - 10) * 2;
		reasons.push('loyal — slow to leave a club');
	}

	// A free agent, or a contract running down, has the door open already.
	if (player.wage === null && player.club === '') {
		score += 35;
		reasons.push('a free agent');
	} else if (expiring) {
		score += 20;
		reasons.push('contract running down');
	}

	// Adaptability eases the upheaval of any move.
	const adapt = lever(player.adaptability);
	if (player.adaptability !== null && adapt > 0.3) {
		score += adapt * 8;
		reasons.push('adaptable — settles quickly');
	}

	score = Math.round(clamp(score, -100, 100));
	return { band: bandOf(score), score, reasons };
}
