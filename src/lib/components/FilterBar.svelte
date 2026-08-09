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
	import { hasAnyFilter } from '$lib/utils/filter';

	const abilityKnown = $derived(scout.summary?.ability_known ?? false);
	const filtered = $derived(hasAnyFilter(scout.filters));
	/** A contract counts as expiring when it ends within a year of the save's
	 * own date (falling back to the system clock when the date is unknown). */
	const expiryCutoff = $derived.by(() => {
		const base = scout.summary?.game_date ?? new Date().toISOString().slice(0, 10);
		const year = Number(base.slice(0, 4)) + 1;
		return `${year}${base.slice(4)}`;
	});
</script>

<div class="border-b border-[var(--color-line)] bg-[var(--color-panel)] px-4 py-3">
	<FilterWho {expiryCutoff} />

	<!-- The numeric bounds only mean anything against people, so the clubs
		tab gets the actions alone. Each band gets its own hairline and
		breathing room, so "who" / "how good" / "do something with it" read
		as three instruments rather than one undifferentiated ribbon. -->
	{#if scout.tab === 'people'}
		<div
			class="mt-2.5 flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-[var(--color-line-soft)] pt-2.5"
		>
			<FilterBounds {abilityKnown} />
			<FilterTraits />
		</div>
	{/if}

	<div
		class="mt-2.5 flex flex-wrap items-center gap-2 border-t border-[var(--color-line-soft)] pt-2.5"
	>
		<FilterActions {filtered} />
	</div>
</div>
