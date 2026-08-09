<script lang="ts">
	/**
	 * What the filter bar does with a result: writes it into one of the save's
	 * own shortlists, saves the search as a preset, or clears the bar.
	 *
	 * The write is the point of the app — filter, then send the results into
	 * the game — so the button carries the count it is about to write and the
	 * backup policy is in its tooltip rather than in a dialog nobody reads.
	 */
	import { scout } from '$lib/classes/Scout.svelte';
	import { savedFilters } from '$lib/classes/SavedFilters.svelte';
	import { describeFilters } from '$lib/utils/filter';

	type Props = {
		/** Whether the bar is filtering anything at all. */
		filtered: boolean;
	};
	const { filtered }: Props = $props();

	let savingPreset = $state(false);
	let presetName = $state('');
	/** Which in-save shortlist the bulk add targets, by name ('' = unnamed). */
	let targetList = $state<string | null>(null);

	const gameLists = $derived(scout.summary?.game_shortlists ?? []);
	/** The selected target, falling back to the first list so the button is
	 * live as soon as a save with shortlists loads. */
	const target = $derived(
		gameLists.find((l) => (l.name ?? '') === targetList) ?? gameLists[0] ?? null
	);
	const nations = $derived(scout.summary?.nations ?? []);
	const nationName = $derived(nations.find((n) => n.id === scout.filters.nationId)?.name);
	const resultCount = $derived(scout.total);

	async function submitPreset(event: SubmitEvent) {
		event.preventDefault();
		await savedFilters.save(presetName, scout.filters);
		presetName = '';
		savingPreset = false;
	}

	async function addResults() {
		if (!target) return;
		await scout.addResultsToGameShortlist(target);
	}
</script>

{#if scout.tab === 'people' && gameLists.length > 0}
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
				class="max-w-32 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
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

<div class="ml-auto flex items-center gap-2">
	{#if savedFilters.presets.length > 0}
		<select
			aria-label="Load a saved filter"
			title="Reapply a filter you saved earlier — the whole bar at once"
			class="max-w-40 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
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
				class="w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
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
