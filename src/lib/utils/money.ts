/**
 * Money as a scout writes it down: short enough for a table cell, and never
 * rounded so far that two different figures read the same.
 *
 * The save stores wages in its own display currency, which Gilet does not
 * convert — the pound sign is FM's own label for a save whose currency is
 * sterling, and converting anything would be inventing a rate.
 */

/** Compact weekly wage: £450K, £8.5K, £400. `blank` is what an unknown wage
 * renders as — an empty cell in a table, an em dash in a panel. */
export function formatWage(wage: number | null, blank = ''): string {
	if (wage === null) return blank;
	if (wage >= 100_000) return `£${Math.round(wage / 1000)}K`;
	if (wage >= 1_000) return `£${(wage / 1000).toFixed(1).replace(/\.0$/, '')}K`;
	return `£${wage}`;
}

/** A club's weekly bill, which runs into millions where one wage does not:
 * £2.4M, £850K, £6,400. */
export function formatBill(total: number | null, blank = ''): string {
	if (total === null) return blank;
	if (total >= 1_000_000) return `£${(total / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`;
	if (total >= 10_000) return `£${Math.round(total / 1000)}K`;
	return `£${total.toLocaleString()}`;
}
