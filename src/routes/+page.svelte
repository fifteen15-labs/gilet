<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import ClubTable from '$lib/components/ClubTable.svelte';
	import CompareBoard from '$lib/components/CompareBoard.svelte';
	import DetailPanel from '$lib/components/DetailPanel.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import PlayerTable from '$lib/components/PlayerTable.svelte';
	import ProfileEditor from '$lib/components/ProfileEditor.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { savedFilters } from '$lib/classes/SavedFilters.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { appVersion, defaultLocations, type Locations } from '$lib/tauri/commands';

	/** Shown in the header so a stale installed build is tellable at a glance
	 * from a fresh one. */
	let version = $state('');
	/** The profile editor replaces the detail panel while it is open — both are
	 * the same right-hand column and only one is useful at a time. */
	let editingProfile = $state(false);
	/** Dialogs open where macOS users expect: saves in FM's own folder.
	 * Resolved in Rust so the paths follow platform convention. */
	let locations = $state<Locations>({ saves: null, documents: null });

	$effect(() => {
		void savedFilters.load();
		void profiles.load();
		void defaultLocations().then((l) => (locations = l));
		void appVersion().then((v) => (version = v));
	});

	async function chooseSave() {
		const picked = await open({
			multiple: false,
			defaultPath: locations.saves ?? undefined,
			filters: [{ name: 'Football Manager save', extensions: ['fm'] }]
		});
		if (typeof picked === 'string') await scout.open(picked);
	}

	const fileName = $derived(scout.summary?.path.split('/').pop() ?? null);
</script>

<div class="flex h-screen flex-col bg-[var(--color-void)]">
	<!-- Padded for the macOS traffic lights, which overlay the window. -->
	<header
		class="flex shrink-0 items-center gap-4 border-b border-[var(--color-line)] px-4 pt-7 pb-3"
		data-tauri-drag-region
	>
		<h1
			class="font-display flex items-center gap-2 text-sm font-semibold tracking-[0.2em] text-[var(--color-bright)] uppercase"
		>
			<img src="/logo.png" alt="" class="h-5 w-5" />
			Gilet
			{#if version}
				<span class="tabular text-[10px] font-normal tracking-normal text-[var(--color-faint)] lowercase">
					v{version}
				</span>
			{/if}
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

		<div class="ml-auto flex items-center gap-2">
			{#if scout.loaded}
				<button
					type="button"
					class="rounded-[2px] border border-[var(--color-line)] px-3 py-1.5 text-xs text-[var(--color-mist)]
						transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
						disabled:cursor-not-allowed disabled:opacity-40"
					title="Read this save again — picks up anything FM has written since"
					disabled={scout.loading}
					onclick={() => scout.reload()}
				>
					Reload
				</button>
			{/if}
			<button
				type="button"
				class="rounded-[2px] border border-[var(--color-line)] px-3 py-1.5 text-xs text-[var(--color-mist)]
					transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]"
				onclick={chooseSave}
			>
				{scout.loaded ? 'Open another save' : 'Open save'}
			</button>
		</div>
	</header>

	<div class="flex min-h-0 flex-1">
		<Sidebar onEditProfile={() => (editingProfile = true)} />

		<main class="flex min-w-0 flex-1 flex-col">
			{#if scout.loading}
				<div class="flex flex-1 items-center justify-center px-8">
					<div class="w-full max-w-sm">
						<p class="eyebrow mb-3">{scout.progressLabel || 'Reading save'}</p>
						<div class="h-[3px] w-full overflow-hidden bg-[var(--color-raised)]">
							<div
								class="h-full bg-[var(--color-signal)] transition-[width] duration-300 ease-out"
								style:width="{Math.round(scout.progress * 100)}%"
							></div>
						</div>
						<p class="tabular mt-2 text-xs text-[var(--color-faint)]">
							{Math.round(scout.progress * 100)}%
						</p>
					</div>
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
								{scout.tab === t.key && !scout.comparing
								? 'border-[var(--color-hivis)] text-[var(--color-bright)]'
								: 'border-transparent text-[var(--color-faint)] hover:text-[var(--color-mist)]'}"
							onclick={() => scout.show(t.key === 'clubs' ? 'clubs' : 'people')}
						>
							{t.label}
						</button>
					{/each}
					<!-- Only offered once something is pinned: an empty board is a
						dead end, and the pin lives in the detail panel. -->
					{#if scout.pinned.length > 0}
						<button
							type="button"
							class="border-b-2 px-3 pb-2 text-xs transition-colors
								{scout.comparing
								? 'border-[var(--color-hivis)] text-[var(--color-bright)]'
								: 'border-transparent text-[var(--color-faint)] hover:text-[var(--color-mist)]'}"
							onclick={() => (scout.comparing = true)}
						>
							Compare ({scout.pinned.length})
						</button>
					{/if}
				</div>

				{#if scout.comparing}
					<CompareBoard />
				{:else}
					<FilterBar />
					{#if scout.tab === 'people'}
						<PlayerTable />
					{:else}
						<ClubTable />
					{/if}
				{/if}
			{/if}

			{#if scout.notice}
				<p class="border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-mist)]">
					{scout.notice}
				</p>
			{/if}
		</main>

		{#if editingProfile}
			<aside class="w-72 shrink-0 border-l border-[var(--color-line)] bg-[var(--color-panel)]">
				<ProfileEditor onClose={() => (editingProfile = false)} />
			</aside>
		{:else}
			<DetailPanel />
		{/if}
	</div>
</div>
