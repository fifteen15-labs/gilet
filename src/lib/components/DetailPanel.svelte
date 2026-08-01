<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import AttributeGrid from './AttributeGrid.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';

	const player = $derived(scout.selectedPlayer);
	const club = $derived(scout.selectedClub);
	const shortlisted = $derived(player !== null && shortlists.activeMembers.has(player.name));
</script>

{#if player || club}
	<aside class="flex w-72 shrink-0 flex-col border-l border-[var(--color-line)] bg-[var(--color-panel)]">
		<div class="flex items-start justify-between gap-2 px-4 pt-4 pb-3">
			<h2 class="font-display text-lg leading-tight text-[var(--color-bright)]">
				{player?.name ?? club?.name}
			</h2>
			<button
				type="button"
				class="text-lg leading-none text-[var(--color-faint)] hover:text-[var(--color-bright)]"
				aria-label="Close details"
				onclick={() => (scout.selectedId = null)}>×</button
			>
		</div>

		<div class="flex-1 overflow-y-auto px-4 pb-4">
			{#if player}
				<dl class="mb-4 grid grid-cols-2 gap-x-4 gap-y-3">
					<div>
						<dt class="eyebrow">Age</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.age}</dd>
					</div>
					<div>
						<dt class="eyebrow">Born</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.born}</dd>
					</div>
					<div>
						<dt class="eyebrow">Current ability</dt>
						<dd class="tabular text-lg text-[var(--color-bright)]">{player.ability ?? '\u2014'}</dd>
					</div>
					<div>
						<dt class="eyebrow">Max ability</dt>
						<dd class="tabular text-lg text-[var(--color-signal)]">{player.potential ?? '\u2014'}</dd>
					</div>
				</dl>

				<div class="mb-4"><AbilityBar ability={player.ability} potential={player.potential} /></div>

				{#if player.positions.length > 0}
					<div class="mb-4">
						<h4 class="eyebrow mb-1.5">Positions</h4>
						<div class="flex flex-wrap gap-1">
							{#each player.positions as pos (pos)}
								<span
									class="rounded-[2px] border border-[var(--color-hivis-dim)] px-1.5 py-0.5 text-xs
										text-[var(--color-hivis)]">{pos}</span
								>
							{/each}
						</div>
					</div>
				{/if}

				<AttributeGrid attributes={player.attributes} />

				<button
					type="button"
					class="mt-5 w-full rounded-[2px] border py-1.5 text-xs transition-colors
						{shortlisted
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}
						disabled:cursor-not-allowed disabled:opacity-40"
					disabled={!shortlists.active}
					onclick={() => shortlists.toggle(player.name)}
				>
					{#if !shortlists.active}
						Create a shortlist first
					{:else if shortlisted}
						Remove from {shortlists.active.name}
					{:else}
						Add to {shortlists.active.name}
					{/if}
				</button>

				<p class="mt-4 border-t border-[var(--color-line)] pt-3 text-xs leading-relaxed text-[var(--color-faint)]">
					Position and club are not decoded from the save format yet.
				</p>
			{:else if club}
				<dl class="space-y-3">
					<div>
						<dt class="eyebrow">Short name</dt>
						<dd class="text-sm text-[var(--color-bright)]">{club.short_name}</dd>
					</div>
					<div>
						<dt class="eyebrow">Club ID</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{club.club_id}</dd>
					</div>
					<div>
						<dt class="eyebrow">Nation ID</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{club.nation_id}</dd>
					</div>
				</dl>
				<p class="mt-5 border-t border-[var(--color-line)] pt-3 text-xs leading-relaxed text-[var(--color-faint)]">
					Squad lists are not decoded yet, so a club cannot be linked to its players. Nation IDs have not
					been resolved to names.
				</p>
			{/if}
		</div>
	</aside>
{/if}
