<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';

	/** Age presets a scout actually uses, rather than a free-form slider. */
	const AGE_PRESETS = [18, 21, 23];
</script>

<div class="flex items-center gap-3 border-b border-[var(--color-line)] px-4 py-2.5">
	<input
		type="search"
		bind:value={scout.filters.query}
		placeholder="Search players"
		aria-label="Search players by name"
		class="w-64 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2.5 py-1.5 text-sm
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>

	<div class="flex items-center gap-1">
		<span class="eyebrow mr-1">Under</span>
		{#each AGE_PRESETS as age (age)}
			<button
				type="button"
				class="tabular rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.maxAge === age
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.maxAge === age}
				onclick={() => (scout.filters.maxAge = scout.filters.maxAge === age ? null : age)}
			>
				{age}
			</button>
		{/each}
	</div>

	<button
		type="button"
		class="rounded-[2px] border px-2 py-1 text-xs transition-colors
			{scout.filters.shortlistedOnly
			? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
			: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
		aria-pressed={scout.filters.shortlistedOnly}
		onclick={() => (scout.filters.shortlistedOnly = !scout.filters.shortlistedOnly)}
	>
		Shortlisted only
	</button>

	<button
		type="button"
		class="ml-auto text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
		onclick={() => scout.reset()}
	>
		Clear filters
	</button>
</div>
