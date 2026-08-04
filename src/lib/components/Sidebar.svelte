<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';

	/** Opens the scoring-profile editor, which takes over the right-hand
	 * column. The page owns that state because the editor and the detail panel
	 * are the same column. */
	type Props = { onEditProfile: () => void };
	const { onEditProfile }: Props = $props();

	/** Members listed under the expanded shortlist before it is summarised. */
	const MEMBER_LIMIT = 50;

	let collapsed = $state(false);
	/** Which in-save list is expanded to show its members, by name key. */
	let openList = $state<string | null>(null);
	/** Clearing writes to the save, so it asks first: this holds the list
	 * awaiting confirmation, by name key. */
	let confirmingClear = $state<string | null>(null);

	const inSave = $derived(scout.summary?.game_shortlists ?? []);

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
			<h2 class="eyebrow" title="The shortlists FM stores inside the loaded save">Shortlists in save</h2>
			<button
				type="button"
				class="text-[var(--color-faint)] hover:text-[var(--color-bright)]"
				aria-label="Collapse shortlists"
				onclick={() => (collapsed = true)}>«</button
			>
		</div>

		<nav class="flex-1 overflow-y-auto px-2">
			{#each inSave as list (list)}
				{@const key = list.name ?? ''}
				<div class="group flex items-center">
					<button
						type="button"
						class="flex flex-1 items-center rounded-[2px] px-2 py-1.5 text-left text-sm transition-colors
							{openList === key
							? 'bg-[var(--color-raised)] text-[var(--color-bright)]'
							: 'text-[var(--color-mist)] hover:text-[var(--color-bright)]'}"
						onclick={() => (openList = openList === key ? null : key)}
					>
						<span class="flex-1 truncate">{list.name ?? '(unnamed)'}</span>
						<span class="tabular ml-1 text-xs text-[var(--color-faint)]">{list.players.length}</span>
					</button>
					{#if list.players.length > 0}
						{#if confirmingClear === key}
							<button
								type="button"
								class="px-1.5 text-xs text-[var(--color-hivis)]"
								title="Remove all {list.players.length} from this shortlist in the save"
								onclick={() => {
									confirmingClear = null;
									void scout.clearGameShortlist(list);
								}}
							>
								Sure?
							</button>
						{:else}
							<button
								type="button"
								class="px-1.5 text-xs text-[var(--color-faint)] opacity-0 transition-opacity
									group-hover:opacity-100 hover:text-[var(--color-hivis)]"
								title="Empty this shortlist in the save"
								onclick={() => (confirmingClear = key)}
							>
								Clear
							</button>
						{/if}
					{/if}
				</div>

				{#if openList === key && list.players.length > 0}
					<ul class="mb-2 ml-3 border-l border-[var(--color-line)] pl-2">
						{#each list.players.slice(0, MEMBER_LIMIT) as member (member)}
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
						{#if list.players.length > MEMBER_LIMIT}
							<li class="px-1 py-0.5 text-xs text-[var(--color-faint)]">
								+{(list.players.length - MEMBER_LIMIT).toLocaleString()} more
							</li>
						{/if}
					</ul>
				{/if}
			{:else}
				<p class="px-2 py-3 text-xs leading-relaxed text-[var(--color-faint)]">
					{scout.loaded
						? 'This save has no shortlists yet. Create one in FM, save, and reopen it here.'
						: 'Open a save to see its shortlists.'}
				</p>
			{/each}
		</nav>

		<div class="border-t border-[var(--color-line)] p-3">
			<h2 class="eyebrow mb-1.5" title="Your own attribute weighting, added as a table column">
				Score by
			</h2>
			<div class="flex items-center gap-1">
				<select
					value={profiles.activeName}
					aria-label="Scoring profile"
					class="min-w-0 flex-1 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
						text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
					onchange={(event) => (profiles.activeName = event.currentTarget.value || null)}
				>
					<option value="">No score</option>
					{#each profiles.list as profile (profile.name)}
						<option value={profile.name}>{profile.name}</option>
					{/each}
				</select>
				<button
					type="button"
					class="rounded-[2px] border border-[var(--color-line)] px-2 py-1 text-xs text-[var(--color-mist)]
						transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
						disabled:cursor-not-allowed disabled:opacity-40"
					disabled={!scout.loaded}
					title={scout.loaded
						? (profiles.active?.name ?? 'Build a scoring profile')
						: 'Open a save first — the editor weights that save’s own attribute names'}
					onclick={onEditProfile}
				>
					{profiles.active ? 'Edit' : 'New'}
				</button>
				{#if profiles.active}
					<button
						type="button"
						class="px-1 text-xs text-[var(--color-faint)] hover:text-[var(--color-hivis)]"
						aria-label="Delete {profiles.active.name}"
						onclick={() => profiles.active && profiles.remove(profiles.active.name)}>×</button
					>
				{/if}
			</div>
			{#if profiles.error}
				<p class="mt-2 text-xs leading-relaxed text-[var(--color-hivis)]">{profiles.error}</p>
			{/if}
		</div>
	</aside>
{/if}
