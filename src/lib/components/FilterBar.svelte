<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';
	import { savedFilters } from '$lib/classes/SavedFilters.svelte';
	import { describeFilters, hasAbilityData, nationsIn } from '$lib/utils/filter';

	let savingPreset = $state(false);
	let presetName = $state('');

	async function submitPreset(event: SubmitEvent) {
		event.preventDefault();
		await savedFilters.save(presetName, scout.filters);
		presetName = '';
		savingPreset = false;
	}

	type Props = { onSaveResults: () => void };
	const { onSaveResults }: Props = $props();

	const abilityKnown = $derived(hasAbilityData(scout.players));
	/** Gender derives from the save's own squads; without women's football it
	 * stays unknown and the filter hides rather than lying. */
	const genderKnown = $derived(scout.players.some((p) => p.female !== null));
	/** A contract counts as expiring when it ends within a year of the save's
	 * own date (falling back to the system clock when the date is unknown). */
	const expiryCutoff = $derived.by(() => {
		const base = scout.summary?.game_date ?? new Date().toISOString().slice(0, 10);
		const year = Number(base.slice(0, 4)) + 1;
		return `${year}${base.slice(4)}`;
	});
	/** Slot order runs back to front, so listing them this way reads like a
	 * team sheet rather than the file's own ordering. */
	const POSITIONS = ['GK', 'DL', 'DC', 'DR', 'WBL', 'WBR', 'DM', 'ML', 'MC', 'MR', 'AML', 'AMC', 'AMR', 'ST'];
	const nations = $derived(nationsIn(scout.players));
	const nationName = $derived(nations.find((n) => n.id === scout.filters.nationId)?.name);
	const resultCount = $derived(scout.results.length);
	const filtered = $derived(
		scout.filters.query.trim() !== '' ||
			scout.filters.maxAge !== null ||
			scout.filters.minAbility !== null ||
			scout.filters.maxAbility !== null ||
			scout.filters.minPotential !== null ||
			scout.filters.maxPotential !== null ||
			scout.filters.kind !== 'all' ||
			scout.filters.position !== null ||
			scout.filters.nationId !== null ||
			scout.filters.gender !== 'all' ||
			scout.filters.contract !== 'any' ||
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

		{#if genderKnown}
			<div class="flex items-center gap-1">
				{#each [{ k: 'all', label: 'Everyone' }, { k: 'men', label: 'Men' }, { k: 'women', label: 'Women' }] as opt (opt.k)}
					<button
						type="button"
						class="rounded-[2px] border px-2 py-1 text-xs transition-colors
							{scout.filters.gender === opt.k
							? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
							: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
						aria-pressed={scout.filters.gender === opt.k}
						onclick={() =>
							(scout.filters.gender = opt.k === 'men' ? 'men' : opt.k === 'women' ? 'women' : 'all')}
					>
						{opt.label}
					</button>
				{/each}
			</div>
		{/if}

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

		<select
			bind:value={scout.filters.nationId}
			aria-label="Filter by nationality"
			class="max-w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
				text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
		>
			<option value={null}>Any nation</option>
			{#each nations as n (n.id)}
				<option value={n.id}>{n.name}</option>
			{/each}
		</select>

		<div class="flex items-center gap-1">
			<span class="eyebrow mr-1">Max age</span>
			<input
				type="number"
				min="14"
				max="60"
				bind:value={scout.filters.maxAge}
				aria-label="Maximum age"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					focus:border-[var(--color-hivis)] focus:outline-none"
			/>
		</div>

		<div
			class="flex items-center gap-1"
			title={abilityKnown ? '' : 'This save has no ability data'}
		>
			<span class="eyebrow mr-1">CA</span>
			<input
				type="number"
				min="1"
				max="200"
				placeholder="min"
				disabled={!abilityKnown}
				bind:value={scout.filters.minAbility}
				aria-label="Minimum current ability"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
					disabled:cursor-not-allowed disabled:opacity-40"
			/>
			<span class="text-xs text-[var(--color-faint)]">–</span>
			<input
				type="number"
				min="1"
				max="200"
				placeholder="max"
				disabled={!abilityKnown}
				bind:value={scout.filters.maxAbility}
				aria-label="Maximum current ability"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
					disabled:cursor-not-allowed disabled:opacity-40"
			/>
			<span class="eyebrow mr-1 ml-2">PA</span>
			<input
				type="number"
				min="1"
				max="200"
				placeholder="min"
				disabled={!abilityKnown}
				bind:value={scout.filters.minPotential}
				aria-label="Minimum potential ability"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
					disabled:cursor-not-allowed disabled:opacity-40"
			/>
			<span class="text-xs text-[var(--color-faint)]">–</span>
			<input
				type="number"
				min="1"
				max="200"
				placeholder="max"
				disabled={!abilityKnown}
				bind:value={scout.filters.maxPotential}
				aria-label="Maximum potential ability"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
					disabled:cursor-not-allowed disabled:opacity-40"
			/>
		</div>

		<div class="flex items-center gap-1">
			<button
				type="button"
				class="rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.contract === 'free'
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.contract === 'free'}
				title="People with no club"
				onclick={() => (scout.filters.contract = scout.filters.contract === 'free' ? 'any' : 'free')}
			>
				Free agents
			</button>
			<button
				type="button"
				class="rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.contract === 'expiring'
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.contract === 'expiring'}
				title="Contract ends within a year of the save's date"
				onclick={() => {
					if (scout.filters.contract === 'expiring') {
						scout.filters.contract = 'any';
						scout.filters.expiryCutoff = null;
					} else {
						scout.filters.contract = 'expiring';
						scout.filters.expiryCutoff = expiryCutoff;
					}
				}}
			>
				Expiring
			</button>
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

		<button
			type="button"
			class="rounded-[2px] border border-[var(--color-line)] px-2 py-1 text-xs text-[var(--color-mist)]
				transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
				disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-[var(--color-line)]
				disabled:hover:text-[var(--color-mist)]"
			disabled={!filtered || resultCount === 0}
			title={shortlists.active ? `Add every result to ${shortlists.active.name}` : 'Creates a first shortlist'}
			onclick={() => shortlists.addAll(scout.results.map((p) => p.name))}
		>
			Add {resultCount.toLocaleString()} to {shortlists.active?.name ?? 'shortlist'}
		</button>
	{/if}

	<div class="ml-auto flex items-center gap-2">
		{#if savedFilters.presets.length > 0}
			<select
				aria-label="Load a saved filter"
				class="max-w-40 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => {
					const name = event.currentTarget.value;
					const preset = savedFilters.get(name);
					if (preset) scout.filters = preset;
					event.currentTarget.value = '';
				}}
			>
				<option value="">Saved filters</option>
				{#each savedFilters.presets as preset (preset.name)}
					<option value={preset.name}>{preset.name}</option>
				{/each}
			</select>
		{/if}

		{#if savingPreset}
			<form onsubmit={submitPreset}>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					autofocus
					bind:value={presetName}
					placeholder="Name this filter"
					class="w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
						placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
					onblur={() => !presetName && (savingPreset = false)}
				/>
			</form>
		{:else if scout.tab === 'people'}
			<button
				type="button"
				class="text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]
					disabled:cursor-not-allowed disabled:opacity-40"
				disabled={!filtered}
				title={filtered ? `Save "${describeFilters(scout.filters, nationName)}"` : 'Set a filter first'}
				onclick={() => {
					presetName = describeFilters(scout.filters, nationName);
					savingPreset = true;
				}}
			>
				Save filter
			</button>
		{/if}

		<button
			type="button"
			class="text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
			onclick={() => scout.reset()}
		>
			Clear filters
		</button>
	</div>
</div>
