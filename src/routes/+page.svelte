<script lang="ts">
	import { open, save } from '@tauri-apps/plugin-dialog';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import PlayerTable from '$lib/components/PlayerTable.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';
	import { exportCsv } from '$lib/tauri/commands';

	let exportError = $state<string | null>(null);

	$effect(() => {
		void shortlists.load();
	});

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
				<span>{scout.players.length.toLocaleString()} people</span>
				<span>{scout.summary?.frames.toLocaleString()} frames</span>
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
		<Sidebar onExport={exportVisible} exportDisabled={!scout.loaded} />

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
				<FilterBar />
				<PlayerTable />
			{/if}

			{#if exportError}
				<p class="border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-hivis)]">
					{exportError}
				</p>
			{/if}
		</main>
	</div>
</div>
