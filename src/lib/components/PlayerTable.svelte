<script lang="ts">
	import PlayerRow from './PlayerRow.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';

	const profile = $derived(profiles.active);
	const total = $derived(scout.results.length);
	const rows = $derived(scout.visibleResults);
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="min-h-0 flex-1 overflow-y-auto">
		<table class="w-full border-collapse">
			<thead class="sticky top-0 z-10 bg-[var(--color-void)]">
				<tr class="border-b border-[var(--color-line)]">
					<th class="pr-4 pb-2 pl-3 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('name')}>
							Name{scout.sortKey === 'name' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-28 pr-4 pb-2 text-left"><span class="eyebrow">Club</span></th>
					<th class="w-32 pr-4 pb-2 text-left"><span class="eyebrow">Position</span></th>
					<th class="w-20 pr-4 pb-2 text-left"><span class="eyebrow">Nation</span></th>
					<th class="w-14 pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('age')}>
							Age{scout.sortKey === 'age' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-20 pr-4 pb-2 text-right"><span class="eyebrow">Wage</span></th>
					<th class="w-16 pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('ability')}>
							CA{scout.sortKey === 'ability' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-16 pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('potential')}>
							PA{scout.sortKey === 'potential' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-16 pr-4 pb-2 text-left">
						<button
							class="eyebrow hover:text-[var(--color-mist)]"
							title="Room left to grow — max ability minus current"
							onclick={() => scout.sortBy('headroom')}
						>
							Grow{scout.sortKey === 'headroom'
								? scout.sortDirection === 'asc'
									? ' ↑'
									: ' ↓'
								: ''}
						</button>
					</th>
					{#if profile}
						<th class="w-16 pr-4 pb-2 text-left">
							<button
								class="eyebrow hover:text-[var(--color-mist)]"
								title="Your weighted average of {profile.name}. Your weights, not an FM figure."
								onclick={() => scout.sortBy('score')}
							>
								{profile.name.slice(0, 8)}{scout.sortKey === 'score'
									? scout.sortDirection === 'asc'
										? ' ↑'
										: ' ↓'
									: ''}
							</button>
						</th>
					{/if}
					<th class="w-16 pr-4 pb-2 text-left">
						<span class="eyebrow" title="Hidden traits worth knowing. Hover a row for the detail.">
							Flags
						</span>
					</th>
					<th class="w-40 pr-3 pb-2 text-left"><span class="eyebrow">Ability / max</span></th>
				</tr>
			</thead>
			<tbody>
				{#each rows as player (player.id)}
					<PlayerRow {player} score={profile ? (scout.scores.get(player.id) ?? null) : undefined} />
				{/each}
			</tbody>
		</table>

		{#if total === 0}
			<p class="px-4 py-8 text-sm text-[var(--color-faint)]">
				Nothing matches those filters. Widen the search or clear the age limit.
			</p>
		{/if}
	</div>

	<footer
		class="flex items-center justify-between border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-faint)]"
	>
		<span class="tabular">
			{#if total > rows.length}
				Showing {rows.length.toLocaleString()} of {total.toLocaleString()} — search to narrow
			{:else}
				{total.toLocaleString()}
				{total === 1 ? 'player' : 'players'}
			{/if}
		</span>
	</footer>
</div>
