<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { describeFilters } from '$lib/utils/filter';

	/** The active tab's snapshot is stale while its state lives on `scout`
	 * directly, so its label reads the live filters instead. */
	function label(index: number): string {
		const search = scout.searches[index];
		if (!search) return '';
		return describeFilters(index === scout.activeSearch ? scout.filters : search.filters);
	}
</script>

<div
	class="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-[var(--color-line)] bg-[var(--color-panel)] px-4 py-1.5"
>
	{#each scout.searches as search, i (search.id)}
		<div
			class="group flex shrink-0 items-center rounded-[2px] border text-xs transition-colors
				{i === scout.activeSearch
				? 'border-[var(--color-hivis)] bg-[var(--color-raised)] text-[var(--color-bright)]'
				: 'border-[var(--color-line)] text-[var(--color-faint)] hover:text-[var(--color-mist)]'}"
		>
			<button
				type="button"
				class="max-w-[24ch] truncate py-1 pr-1 pl-2.5"
				title={label(i)}
				onclick={() => scout.switchSearch(i)}
			>
				{label(i)}
			</button>
			<button
				type="button"
				class="px-1.5 py-1 text-[var(--color-faint)] opacity-0 transition-opacity
					group-hover:opacity-100 hover:text-[var(--color-hivis)]
					{i === scout.activeSearch ? 'opacity-100' : ''}"
				title="Close this search"
				aria-label="Close this search"
				onclick={() => scout.closeSearch(i)}
			>
				×
			</button>
		</div>
	{/each}
	<button
		type="button"
		class="shrink-0 rounded-[2px] border border-[var(--color-line)] px-2 py-1 text-xs text-[var(--color-faint)]
			transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]"
		title="New search — the current one keeps its filters and stays open"
		onclick={() => scout.newSearch()}
	>
		+
	</button>
</div>
