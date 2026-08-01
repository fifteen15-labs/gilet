<script lang="ts">
	import { open, save } from '@tauri-apps/plugin-dialog';
	import ClubTable from '$lib/components/ClubTable.svelte';
	import DetailPanel from '$lib/components/DetailPanel.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import PlayerTable from '$lib/components/PlayerTable.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';
	import { exportCsv, importCsv } from '$lib/tauri/commands';
	import { describeFilters } from '$lib/utils/filter';

	let exportError = $state<string | null>(null);
	let notice = $state<string | null>(null);

	$effect(() => {
		void shortlists.load();
	});

	/** Keeps the current filtered results as a new shortlist, named after the
	 * search that produced them. */
	async function saveResultsAsShortlist() {
		exportError = null;
		const rows = scout.matching(shortlists.activeMembers);
		const name = await shortlists.saveAs(
			describeFilters(scout.filters),
			rows.map((p) => p.name)
		);
		notice = name ? `Saved ${rows.length.toLocaleString()} players as "${name}".` : null;
	}

	/** Reads names from a CSV into the active shortlist. */
	async function importIntoShortlist() {
		exportError = null;
		notice = null;
		const list = shortlists.active;
		if (!list) {
			notice = 'Create a shortlist first, then import into it.';
			return;
		}
		const picked = await open({ multiple: false, filters: [{ name: 'CSV', extensions: ['csv'] }] });
		if (typeof picked !== 'string') return;
		try {
			const known = scout.players.map((p) => p.name);
			const result = await importCsv(picked, known);
			for (const name of result.matched) {
				if (!list.players.includes(name)) await shortlists.toggle(name);
			}
			notice =
				result.unmatched.length > 0
					? `Added ${result.matched.length}. ${result.unmatched.length} not found in this save: ${result.unmatched.slice(0, 3).join(', ')}${result.unmatched.length > 3 ? '…' : ''}`
					: `Added ${result.matched.length} to ${list.name}.`;
		} catch (e) {
			exportError = e instanceof Error ? e.message : String(e);
		}
	}

	async function chooseSave() {
		const picked = await open({
			multiple: false,
			filters: [{ name: 'Football Manager save', extensions: ['fm'] }]
		});
		if (typeof picked === 'string') await scout.open(picked);
	}

	/** Exports what is on screen: the active shortlist if one is selected,
	 * otherwise the current filtered set. */
	async function exportVisible() {
		exportError = null;
		notice = null;
		const rows = scout.matching(shortlists.activeMembers);
		const suggested = shortlists.active ? `${shortlists.active.name}.csv` : 'players.csv';
		const target = await save({ defaultPath: suggested, filters: [{ name: 'CSV', extensions: ['csv'] }] });
		if (!target) return;
		try {
			await exportCsv(target, $state.snapshot(rows));
		} catch (e) {
			exportError = e instanceof Error ? e.message : String(e);
		}
	}

	const fileName = $derived(scout.summary?.path.split('/').pop() ?? null);
</script>

<div class="flex h-screen flex-col bg-[var(--color-void)]">
	<!-- Padded for the macOS traffic lights, which overlay the window. -->
	<header
		class="flex shrink-0 items-center gap-4 border-b border-[var(--color-line)] px-4 pt-7 pb-3"
		data-tauri-drag-region
	>
		<h1 class="font-display text-sm font-semibold tracking-[0.2em] text-[var(--color-bright)] uppercase">
			Anorak
		</h1>

		{#if scout.loaded}
			<p class="tabular flex items-center gap-3 text-xs text-[var(--color-faint)]">
				<span class="text-[var(--color-mist)]">{fileName}</span>
				{#if scout.summary?.game_date}
					<span title="Ages are calculated against the save's own date">
						{scout.summary.game_date}
					</span>
				{:else}
					<span title="This save's in-game date could not be read, so ages use today's date">
						date unknown
					</span>
				{/if}
				<span>{scout.players.length.toLocaleString()} people</span>
				<span>{scout.summary?.parse_millis}ms</span>
			</p>
		{/if}

		<button
			type="button"
			class="ml-auto rounded-[2px] border border-[var(--color-line)] px-3 py-1.5 text-xs text-[var(--color-mist)]
				transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]"
			onclick={chooseSave}
		>
			{scout.loaded ? 'Open another save' : 'Open save'}
		</button>
	</header>

	<div class="flex min-h-0 flex-1">
		<Sidebar
			onExport={exportVisible}
			onImport={importIntoShortlist}
			exportDisabled={!scout.loaded}
		/>

		<main class="flex min-w-0 flex-1 flex-col">
			{#if scout.loading}
				<div class="flex flex-1 items-center justify-center">
					<p class="eyebrow">Reading save</p>
				</div>
			{:else if scout.error}
				<div class="flex flex-1 items-center justify-center px-8">
					<div class="max-w-md">
						<h2 class="eyebrow mb-2 text-[var(--color-hivis)]">Could not read that file</h2>
						<p class="text-sm leading-relaxed text-[var(--color-mist)]">{scout.error}</p>
					</div>
				</div>
			{:else if !scout.loaded}
				<div class="flex flex-1 items-center justify-center px-8">
					<div class="max-w-sm">
						<h2 class="font-display mb-2 text-2xl text-[var(--color-bright)]">Open a save to begin</h2>
						<p class="mb-6 text-sm leading-relaxed text-[var(--color-mist)]">
							Saves live in Library → Application Support → Sports Interactive → Football Manager 26 → games.
						</p>
						<button
							type="button"
							class="rounded-[2px] bg-[var(--color-hivis)] px-4 py-2 text-sm font-medium text-[var(--color-void)]
								transition-opacity hover:opacity-90"
							onclick={chooseSave}
						>
							Open save
						</button>
					</div>
				</div>
			{:else}
				<div class="flex items-center gap-1 border-b border-[var(--color-line)] px-4 pt-2">
					{#each [{ key: 'people', label: `People (${scout.players.length.toLocaleString()})` }, { key: 'clubs', label: `Clubs (${scout.clubs.length.toLocaleString()})` }] as t (t.key)}
						<button
							type="button"
							class="border-b-2 px-3 pb-2 text-xs transition-colors
								{scout.tab === t.key
								? 'border-[var(--color-hivis)] text-[var(--color-bright)]'
								: 'border-transparent text-[var(--color-faint)] hover:text-[var(--color-mist)]'}"
							onclick={() => scout.show(t.key === 'clubs' ? 'clubs' : 'people')}
						>
							{t.label}
						</button>
					{/each}
				</div>

				<FilterBar onSaveResults={saveResultsAsShortlist} />
				{#if scout.tab === 'people'}
					<PlayerTable />
				{:else}
					<ClubTable />
				{/if}
			{/if}

			{#if exportError}
				<p class="border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-hivis)]">
					{exportError}
				</p>
			{:else if notice}
				<p class="border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-mist)]">
					{notice}
				</p>
			{/if}
		</main>

		<DetailPanel />
	</div>
</div>
