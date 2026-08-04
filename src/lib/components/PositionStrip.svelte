<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { ACCOMPLISHED, NATURAL, PITCH } from '$lib/utils/positions';

	type Props = {
		/** The 15 slot ratings, 1-20, in the save's own slot order. */
		ratings: number[];
	};
	const { ratings }: Props = $props();

	/** Slot labels come from the save rather than a copy kept here. */
	const names = $derived(scout.summary?.position_names ?? []);

	const cells = $derived(
		PITCH.map((cell) => ({
			...cell,
			label: names[cell.slot] ?? '',
			value: ratings[cell.slot] ?? null
		})).filter((cell) => cell.label !== '')
	);

	/**
	 * Three states, not a gradient: a position the player plays, one they can
	 * be asked to fill, and one they cannot. A continuous ramp reads as
	 * precision the 1-20 scale does not have at the bottom end — 4 and 7 mean
	 * the same thing in practice, which is "no".
	 */
	function shade(value: number | null): string {
		if (value === null) return 'border-[var(--color-line-soft)] text-[var(--color-faint)]';
		if (value >= NATURAL)
			return 'border-[var(--color-signal)] bg-[var(--color-signal-dim)] text-[var(--color-bright)]';
		if (value >= ACCOMPLISHED) return 'border-[var(--color-signal-dim)] text-[var(--color-signal)]';
		return 'border-[var(--color-line-soft)] text-[var(--color-faint)]';
	}
</script>

{#if ratings.length > 0}
	<div class="grid grid-cols-5 gap-1">
		{#each cells as cell (cell.slot)}
			<div
				class="flex flex-col items-center justify-center rounded-[2px] border py-1 {shade(cell.value)}"
				style="grid-row: {cell.row}; grid-column: {cell.column}"
				title="{cell.label} {cell.value ?? '—'}/20"
			>
				<span class="font-display text-[10px] leading-none tracking-wider">{cell.label}</span>
				<span class="tabular text-xs leading-tight">{cell.value ?? '–'}</span>
			</div>
		{/each}
	</div>
	<p class="mt-1.5 text-xs leading-relaxed text-[var(--color-faint)]">
		How naturally the player takes each position, 1&ndash;20. Filled is {NATURAL}+, the line FM
		itself treats as a position they play; outlined is {ACCOMPLISHED}+, one they can be asked to
		fill.
	</p>
{/if}
