<script lang="ts">
	import { shortlists } from '$lib/classes/Shortlists.svelte';
	import { scout } from '$lib/classes/Scout.svelte';

	type Props = { onExport: () => void; onImport: () => void; exportDisabled: boolean };
	const { onExport, onImport, exportDisabled }: Props = $props();

	let draftName = $state('');
	let adding = $state(false);
	let collapsed = $state(false);
	/** List currently being renamed, or null. */
	let renaming = $state<string | null>(null);
	let renameDraft = $state('');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		await shortlists.create(draftName);
		draftName = '';
		adding = false;
	}

	async function submitRename(event: SubmitEvent) {
		event.preventDefault();
		if (renaming !== null) {
			await shortlists.rename(renaming, renameDraft);
		}
		renaming = null;
		renameDraft = '';
	}

	/** Selecting a member in the sidebar opens them in the detail panel. */
	function open(name: string) {
		const player = scout.players.find((p) => p.name === name);
		if (player) {
			scout.tab = 'people';
			scout.selectedId = player.id;
		}
	}
</script>

{#if collapsed}
	<aside class="flex w-9 shrink-0 border-r border-[var(--color-line)] bg-[var(--color-panel)]">
		<button
			type="button"
			class="flex w-full items-start justify-center pt-4 text-[var(--color-faint)] hover:text-[var(--color-bright)]"
			aria-label="Expand shortlists"
			title="Shortlists"
			onclick={() => (collapsed = false)}>»</button
		>
	</aside>
{:else}
	<aside class="flex w-56 shrink-0 flex-col border-r border-[var(--color-line)] bg-[var(--color-panel)]">
		<div class="flex items-center justify-between px-4 pt-4 pb-2">
			<h2 class="eyebrow">Shortlists</h2>
			<div class="flex items-center gap-2">
				<button
					type="button"
					class="text-lg leading-none text-[var(--color-faint)] hover:text-[var(--color-hivis)]"
					aria-label="New shortlist"
					onclick={() => (adding = true)}>+</button
				>
				<button
					type="button"
					class="text-[var(--color-faint)] hover:text-[var(--color-bright)]"
					aria-label="Collapse shortlists"
					onclick={() => (collapsed = true)}>«</button
				>
			</div>
		</div>

		{#if adding}
			<form class="px-3 pb-2" onsubmit={submit}>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					autofocus
					bind:value={draftName}
					placeholder="Name this list"
					class="w-full rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-sm
						placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
					onblur={() => !draftName && (adding = false)}
				/>
			</form>
		{/if}

		<nav class="flex-1 overflow-y-auto px-2">
			{#each shortlists.lists as list (list.name)}
				<div class="group flex items-center">
					{#if renaming === list.name}
						<form class="flex-1 px-1 py-1" onsubmit={submitRename}>
							<!-- svelte-ignore a11y_autofocus -->
							<input
								autofocus
								bind:value={renameDraft}
								class="w-full rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-sm
									focus:border-[var(--color-hivis)] focus:outline-none"
								onblur={() => (renaming = null)}
							/>
						</form>
					{:else}
						<button
							type="button"
							class="flex-1 truncate rounded-[2px] px-2 py-1.5 text-left text-sm transition-colors
								{shortlists.activeName === list.name
								? 'bg-[var(--color-raised)] text-[var(--color-bright)]'
								: 'text-[var(--color-mist)] hover:text-[var(--color-bright)]'}"
							title="Double-click to rename"
							onclick={() => (shortlists.activeName = list.name)}
							ondblclick={() => {
								renaming = list.name;
								renameDraft = list.name;
							}}
						>
							{#if shortlists.activeName === list.name}
								<span class="mr-1.5 text-[var(--color-hivis)]">▍</span>
							{/if}
							{list.name}
							<span class="tabular ml-1 text-xs text-[var(--color-faint)]">{list.players.length}</span>
						</button>
						<button
							type="button"
							class="px-2 text-[var(--color-faint)] opacity-0 group-hover:opacity-100 hover:text-[var(--color-hivis)]"
							aria-label="Delete {list.name}"
							onclick={() => shortlists.remove(list.name)}>×</button
						>
					{/if}
				</div>

				{#if shortlists.activeName === list.name && list.players.length > 0}
					<ul class="mb-2 ml-3 border-l border-[var(--color-line)] pl-2">
						{#each list.players as member (member)}
							<li>
								<button
									type="button"
									class="w-full truncate rounded-[2px] px-1 py-0.5 text-left text-xs text-[var(--color-faint)]
										hover:text-[var(--color-bright)]"
									onclick={() => open(member)}
								>
									{member}
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			{:else}
				<p class="px-2 py-3 text-xs leading-relaxed text-[var(--color-faint)]">
					No shortlists yet. Create one, then tick players to add them.
				</p>
			{/each}
		</nav>

		<div class="space-y-2 border-t border-[var(--color-line)] p-3">
			<button
				type="button"
				class="w-full rounded-[2px] border border-[var(--color-line)] py-1.5 text-xs text-[var(--color-mist)]
					transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
					disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-[var(--color-line)]
					disabled:hover:text-[var(--color-mist)]"
				disabled={exportDisabled}
				onclick={onImport}
			>
				Import CSV
			</button>
			<button
				type="button"
				class="w-full rounded-[2px] border border-[var(--color-line)] py-1.5 text-xs text-[var(--color-mist)]
					transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
					disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-[var(--color-line)]
					disabled:hover:text-[var(--color-mist)]"
				disabled={exportDisabled}
				onclick={onExport}
			>
				Export CSV
			</button>
			{#if shortlists.error}
				<p class="mt-2 text-xs text-[var(--color-hivis)]">{shortlists.error}</p>
			{/if}
		</div>
	</aside>
{/if}
