<script lang="ts">
	/**
	 * The filter bar, in two rows: who you are looking for, then how good they
	 * have to be. The split keeps the bar scannable as filters accumulate, and
	 * each group owns its own controls — this file holds only what they need to
	 * agree on and the shape they sit in.
	 */
	import FilterActions from './FilterActions.svelte';
	import FilterBounds from './FilterBounds.svelte';
	import FilterTraits from './FilterTraits.svelte';
	import FilterWho from './FilterWho.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { hasAbilityData, hasAnyFilter } from '$lib/utils/filter';

	const abilityKnown = $derived(hasAbilityData(scout.players));
	const filtered = $derived(hasAnyFilter(scout.filters));
	/** A contract counts as expiring when it ends within a year of the save's
	 * own date (falling back to the system clock when the date is unknown). */
	const expiryCutoff = $derived.by(() => {
		const base = scout.summary?.game_date ?? new Date().toISOString().slice(0, 10);
		const year = Number(base.slice(0, 4)) + 1;
		return `${year}${base.slice(4)}`;
	});
</script>

<div class="border-b border-[var(--color-line)] px-4 py-2">
	<FilterWho {expiryCutoff} {abilityKnown} />

	<div class="mt-2 flex flex-wrap items-center gap-3">
		<!-- The numeric bounds only mean anything against people, so the clubs
			tab gets the actions alone. -->
		{#if scout.tab === 'people'}
			<FilterBounds {abilityKnown} />
			<FilterTraits />
		{/if}
		<FilterActions {filtered} />
	</div>
</div>
