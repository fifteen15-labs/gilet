<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';
	import { hasAbilityData } from '$lib/utils/filter';

	type Props = { onSaveResults: () => void };
	const { onSaveResults }: Props = $props();

	/** Age brackets a scout actually uses, rather than a free-form slider. */
	const AGE_PRESETS = [18, 21, 23];

	const abilityKnown = $derived(hasAbilityData(scout.players));
	/** Slot order runs back to front, so listing them this way reads like a
	 * team sheet rather than the file's own ordering. */
	const POSITIONS = ['GK', 'DL', 'DC', 'DR', 'WBL', 'WBR', 'DM', 'ML', 'MC', 'MR', 'AML', 'AMC', 'AMR', 'ST'];
	const resultCount = $derived(scout.matching(shortlists.activeMembers).length);
	const filtered = $derived(
		scout.filters.query.trim() !== '' ||
			scout.filters.maxAge !== null ||
			scout.filters.minAbility !== null ||
			scout.filters.minPotential !== null ||
			scout.filters.kind !== 'all' ||
			scout.filters.position !== null ||
			scout.filters.shortlistedOnly
	);
</script>

<div class="flex flex-wrap items-center gap-3 border-b border-[var(--color-line)] px-4 py-2.5">
	<input
		type="search"
		bind:value={scout.filters.query}
		placeholder={scout.tab === 'clubs' ? 'Search clubs' : 'Search players'}
		aria-label="Search by name"
		class="w-56 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2.5 py-1.5 text-sm
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>

	{#if scout.tab === 'people'}
		<div class="flex items-center gap-1">
			{#each [{ k: 'all', label: 'All' }, { k: 'players', label: 'Players' }, { k: 'staff', label: 'Staff' }] as opt (opt.k)}
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.kind === opt.k
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.kind === opt.k}
					onclick={() => (scout.filters.kind = opt.k === 'players' ? 'players' : opt.k === 'staff' ? 'staff' : 'all')}
				>
					{opt.label}
				</button>
			{/each}
		</div>

		<select
			bind:value={scout.filters.position}
			aria-label="Filter by position"
			class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
				text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
		>
			<option value={null}>Any position</option>
			{#each POSITIONS as p (p)}
				<option value={p}>{p}</option>
			{/each}
		</select>

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

		<div
			class="flex items-center gap-1"
			title={abilityKnown ? '' : 'This save has no ability data'}
		>
			<span class="eyebrow mr-1">CA over</span>
			<input
				type="number"
				min="1"
				max="200"
				disabled={!abilityKnown}
				bind:value={scout.filters.minAbility}
				aria-label="Minimum current ability"
				class="tabular w-16 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					focus:border-[var(--color-hivis)] focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
			/>
			<span class="eyebrow mr-1 ml-2">PA over</span>
			<input
				type="number"
				min="1"
				max="200"
				disabled={!abilityKnown}
				bind:value={scout.filters.minPotential}
				aria-label="Minimum potential ability"
				class="tabular w-16 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					focus:border-[var(--color-hivis)] focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
			/>
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
			class="rounded-[2px] border border-[var(--color-line)] px-2 py-1 text-xs text-[var(--color-mist)]
				transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
				disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-[var(--color-line)]
				disabled:hover:text-[var(--color-mist)]"
			disabled={!filtered || resultCount === 0}
			onclick={onSaveResults}
		>
			Save {resultCount.toLocaleString()} as shortlist
		</button>
	{/if}

	<button
		type="button"
		class="ml-auto text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
		onclick={() => scout.reset()}
	>
		Clear filters
	</button>
</div>
