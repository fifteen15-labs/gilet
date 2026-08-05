<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { savedFilters } from '$lib/classes/SavedFilters.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { describeFilters, hasAbilityData, nationsIn, type Filters } from '$lib/utils/filter';
	import { hasFlagData } from '$lib/utils/flags';
	import { SET_PIECES } from '$lib/utils/attributes';
	import { POSITIONS } from '$lib/utils/positions';

	let savingPreset = $state(false);
	let presetName = $state('');
	/** Which in-save shortlist the bulk add targets, by name ('' = unnamed). */
	let targetList = $state<string | null>(null);

	async function submitPreset(event: SubmitEvent) {
		event.preventDefault();
		await savedFilters.save(presetName, scout.filters);
		presetName = '';
		savingPreset = false;
	}

	const gameLists = $derived(scout.summary?.game_shortlists ?? []);
	/** The selected target, falling back to the first list so the button is
	 * live as soon as a save with shortlists loads. */
	const target = $derived(
		gameLists.find((l) => (l.name ?? '') === targetList) ?? gameLists[0] ?? null
	);

	async function addResults() {
		if (!target) return;
		await scout.addResultsToGameShortlist(target);
	}

	function setStaffRole(value: string) {
		scout.filters.staffRole = value as Filters['staffRole'];
	}

	const abilityKnown = $derived(hasAbilityData(scout.players));
	/** The flag rules read hidden attributes and the personality run; without
	 * either there is nothing to flag, so the toggles hide rather than filter
	 * everyone out. */
	const flagsKnown = $derived(scout.players.some(hasFlagData));
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
			scout.filters.staffRole !== 'any' ||
			scout.filters.position !== null ||
			scout.filters.nationId !== null ||
			scout.filters.minScore !== null ||
			scout.filters.gender !== 'all' ||
			scout.filters.contract !== 'any' ||
			scout.filters.minHeadroom !== null ||
			scout.filters.risk !== 'any' ||
			scout.filters.maxWage !== null ||
			scout.filters.minVersatility !== null ||
			scout.filters.minPositions !== null ||
			scout.filters.setPiece !== null
	);

	/**
	 * The bargain board: players whose contract is running down, with real room
	 * left to grow and nothing on the report to explain why they are cheap.
	 * Ordered by headroom, because that is what the search is actually about.
	 *
	 * No wage cap is baked in. Wages differ by orders of magnitude between a
	 * save's top division and its fifth, so any figure hard-coded here would be
	 * wrong for most saves — the Max wage box next to it is the lever, and it
	 * is left where the user can see it.
	 */
	function bargains() {
		scout.screen(
			{
				kind: 'players',
				contract: 'expiring',
				expiryCutoff,
				minHeadroom: 20,
				risk: 'clean'
			},
			'headroom'
		);
	}
</script>

<!-- Two rows: who you are looking for, then how good they have to be. The
	split keeps the bar scannable as filters accumulate. -->
<div class="border-b border-[var(--color-line)] px-4 py-2">
	<div class="flex flex-wrap items-center gap-3">
	<input
		type="search"
		bind:value={scout.filters.query}
		placeholder={scout.tab === 'clubs' ? 'Search clubs' : 'Search players'}
		aria-label="Search by name"
		title="Search by name or club — accents don't matter, so 'mbappe' finds Mbappé and 'man city' lists City's squad"
		class="w-56 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2.5 py-1.5 text-sm
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>

	{#if scout.tab === 'people'}
		<div class="flex items-center gap-1">
			{#each [
				{ k: 'all', label: 'All', tip: 'Everyone in the save — players, staff, and undecoded squad fillers' },
				{ k: 'players', label: 'Players', tip: 'Only people with a player attribute block' },
				{ k: 'staff', label: 'Staff', tip: 'Only non-players — coaches, physios, scouts, managers. Unlocks the role filter, and the CA/PA bounds read their non-player ability' }
			] as opt (opt.k)}
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.kind === opt.k
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.kind === opt.k}
					title={opt.tip}
					onclick={() => (scout.filters.kind = opt.k === 'players' ? 'players' : opt.k === 'staff' ? 'staff' : 'all')}
				>
					{opt.label}
				</button>
			{/each}
		</div>

		{#if scout.filters.kind === 'staff'}
			<select
				value={scout.filters.staffRole}
				aria-label="Backroom role"
				title="The save's own backroom groups: the manager seat, or the coaching, medical and recruitment department lists. Staff the save gives no department for only show under Any role."
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => setStaffRole(event.currentTarget.value)}
			>
				<option value="any">Any role</option>
				<option value="Manager">Manager</option>
				<option value="Coaching">Coaching</option>
				<option value="Medical">Medical</option>
				<option value="Recruitment">Recruitment</option>
			</select>
		{/if}

		{#if genderKnown}
			<div
				class="flex items-center gap-1"
				title="Gender derives from the save's own squads. Anyone the save can't settle only shows under Everyone — showing a woman under Men would be a guess"
			>
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
			title="Only players comfortable in this position — rated a natural. Staff have no positions, so any position filter hides them"
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
			title="By nationality, using the save's own numbering. Nations the parser hasn't named yet appear as raw identifiers at the bottom — they still filter correctly"
			class="max-w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
				text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
		>
			<option value={null}>Any nation</option>
			{#each nations as n (n.id)}
				<option value={n.id}>{n.name}</option>
			{/each}
		</select>

		<div class="flex items-center gap-1">
			<button
				type="button"
				class="rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.contract === 'free'
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.contract === 'free'}
				title="No contract, no wage, no club. For staff — whose wages aren't decoded — this reads as 'no club found', so a few obscure employed staff can slip in"
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
				title="Contract ends within a year of the save's own date. Contracts the parser couldn't read never count as expiring — an unknown deal is not a bargain"
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

		{#if flagsKnown}
			<div class="flex items-center gap-1">
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.risk === 'clean'
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.risk === 'clean'}
					title="Only players the save can vouch for — no injury proneness, no temperament or professionalism problems. Staff and undecoded stubs drop out: no reading is not a good one."
					onclick={() => (scout.filters.risk = scout.filters.risk === 'clean' ? 'any' : 'clean')}
				>
					No red flags
				</button>
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.risk === 'flagged'
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.risk === 'flagged'}
					title="Only players carrying at least one red flag — for auditing a squad you already own"
					onclick={() => (scout.filters.risk = scout.filters.risk === 'flagged' ? 'any' : 'flagged')}
				>
					Red flags
				</button>
			</div>
		{/if}

		<button
			type="button"
			class="rounded-[2px] border border-[var(--color-signal-dim)] px-2 py-1 text-xs text-[var(--color-signal)]
				transition-colors hover:border-[var(--color-signal)]
				disabled:cursor-not-allowed disabled:opacity-40"
			disabled={!abilityKnown}
			title={abilityKnown
				? 'Clears the bar and searches for players with a contract expiring within a year, at least 20 points of room to grow, and no red flags — sorted by room to grow. Everything it sets stays editable here; add a wage cap to suit your budget.'
				: 'This save has no ability data, and the search is built on room to grow'}
			onclick={bargains}
		>
			Bargains
		</button>
	{/if}
	</div>

	<div class="mt-2 flex flex-wrap items-center gap-3">
	{#if scout.tab === 'people'}
		<div class="flex items-center gap-1">
			<span class="eyebrow mr-1" title="Highest weekly wage. Players whose wage the parser could not read are excluded — an unreadable wage is not a cheap one.">
				Max wage
			</span>
			<input
				type="number"
				min="0"
				step="500"
				bind:value={scout.filters.maxWage}
				aria-label="Maximum weekly wage"
				class="tabular w-20 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					focus:border-[var(--color-hivis)] focus:outline-none"
			/>
		</div>

		<div
			class="flex items-center gap-1"
			title="Age on the save's own in-game date. People with no readable birth date — stubs and compacted people — never pass an age cap: an unknown age is not a young one"
		>
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
			<span
				class="eyebrow mr-1"
				title="Current Ability, 1-200 — a player's own, or the non-player CA for staff. Both are the save's exact figures. Anyone with neither decoded fails the bound rather than passing at zero"
			>
				CA
			</span>
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
			<span
				class="eyebrow mr-1 ml-2"
				title="Potential Ability, 1-200 — the ceiling the save assigns, player or staff. A max bound rules out the ones already at the top"
			>
				PA
			</span>
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
			<span
				class="eyebrow mr-1 ml-2"
				title="Room left to grow: max ability minus current. The development screener's one number."
			>
				Grow
			</span>
			<input
				type="number"
				min="0"
				max="200"
				placeholder="min"
				disabled={!abilityKnown}
				bind:value={scout.filters.minHeadroom}
				aria-label="Minimum headroom, max ability minus current"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
					disabled:cursor-not-allowed disabled:opacity-40"
			/>
		</div>

		<div class="flex items-center gap-1">
			<span
				class="eyebrow mr-1"
				title="How many positions the player is already rated 15+ in — what they can cover on Saturday"
			>
				Covers
			</span>
			<input
				type="number"
				min="1"
				max="15"
				placeholder="min"
				bind:value={scout.filters.minPositions}
				aria-label="Minimum positions covered"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
			/>
			<span
				class="eyebrow mr-1 ml-2"
				title="The Versatility attribute — how readily the player learns a new role, which is not the same as how many they already play"
			>
				Versatility
			</span>
			<input
				type="number"
				min="1"
				max="20"
				placeholder="min"
				bind:value={scout.filters.minVersatility}
				aria-label="Minimum versatility"
				class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
			/>
		</div>

		<div class="flex items-center gap-1">
			<select
				value={scout.filters.setPiece}
				aria-label="Filter by a set-piece skill"
				title="Filter on one dead-ball skill — corners, free kicks, penalties, long throws. Picking one arms the minimum at 14, a specialist's level; the box stays editable"
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => {
					const key = event.currentTarget.value;
					scout.filters.setPiece = key === '' ? null : key;
					// A skill with no minimum filters nothing, so picking one
					// arms it at a specialist's level; the box stays editable.
					if (key !== '' && scout.filters.minSetPiece === null) scout.filters.minSetPiece = 14;
					if (key === '') scout.filters.minSetPiece = null;
				}}
			>
				<option value="">Any set piece</option>
				{#each SET_PIECES as skill (skill.key)}
					<option value={skill.key}>{skill.label}</option>
				{/each}
			</select>
			{#if scout.filters.setPiece !== null}
				<input
					type="number"
					min="1"
					max="20"
					placeholder="min"
					bind:value={scout.filters.minSetPiece}
					aria-label="Minimum set-piece rating"
					class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
						placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
				/>
			{/if}
		</div>

		{#if profiles.active}
			<div class="flex items-center gap-1">
				<span
					class="eyebrow mr-1"
					title="Minimum score under your active profile — a weighted average of the attributes you chose, on the same 1-20 scale. Your weights, not an FM figure; anyone missing the sheet it weights is excluded"
				>
					{profiles.active.name}
				</span>
				<input
					type="number"
					min="1"
					max="20"
					step="0.5"
					placeholder="min"
					bind:value={scout.filters.minScore}
					aria-label="Minimum score"
					class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
						placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
				/>
			</div>
		{/if}

		{#if gameLists.length > 0}
			<div
				class="flex items-center gap-1"
				title={scout.canEditGameShortlists
					? 'Writes the results into the save file — FM sees them on next load. The untouched original is kept as a .gilet.bak sibling.'
					: "Read-only: this save's own date could not be read"}
			>
				<button
					type="button"
					class="rounded-[2px] border border-[var(--color-hivis-dim)] px-2 py-1 text-xs text-[var(--color-hivis)]
						transition-colors hover:border-[var(--color-hivis)]
						disabled:cursor-not-allowed disabled:opacity-40"
					disabled={!filtered || resultCount === 0 || !scout.canEditGameShortlists || !target}
					onclick={addResults}
				>
					Add {resultCount.toLocaleString()} to save
				</button>
				{#if gameLists.length > 1}
					<select
						aria-label="Which in-save shortlist to add to"
						title="Which of FM's own in-save shortlists the button writes into"
						class="max-w-32 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
							text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
						onchange={(event) => (targetList = event.currentTarget.value)}
					>
						{#each gameLists as list (list)}
							<option value={list.name ?? ''} selected={list === target}>
								{list.name ?? '(unnamed)'}
							</option>
						{/each}
					</select>
				{:else if target}
					<span class="text-xs text-[var(--color-faint)]">→ {target.name ?? '(unnamed)'}</span>
				{/if}
			</div>
		{/if}
	{/if}

	<div class="ml-auto flex items-center gap-2">
		{#if savedFilters.presets.length > 0}
			<select
				aria-label="Load a saved filter"
				title="Reapply a filter you saved earlier — the whole bar at once"
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
			title="Back to the whole save — clears every filter on the bar"
			onclick={() => scout.reset()}
		>
			Clear filters
		</button>
	</div>
	</div>
</div>
