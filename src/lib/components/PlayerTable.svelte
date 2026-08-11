<script lang="ts">
	import PlayerRow from './PlayerRow.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';

	const profile = $derived(profiles.active);
	const total = $derived(scout.total);
	const rows = $derived(scout.rows);
	/** The shared column carries positions for players and the backroom role
	 * for staff; the header follows the kind filter so it names what the
	 * rows actually show. */
	const positionHeader = $derived(
		scout.filters.kind === 'staff'
			? 'Role'
			: scout.filters.kind === 'players'
				? 'Position'
				: 'Position / Role'
	);
	/** Worldwide reputation now reads for players as well as staff, so the
	 * column earns its width for either kind. It hides only under "All", where
	 * it would sit against undecoded stubs and compact entries with nothing to
	 * show — unless the table is sorted by it, since a sort on a column nobody
	 * can see reads as no sort at all. */
	const showReputation = $derived(
		scout.filters.kind === 'staff' || scout.filters.kind === 'players' || scout.sortKey === 'reputation'
	);
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
					<th class="w-32 pr-4 pb-2 text-left"
						><span class="eyebrow" title="Positions for players; the backroom role for staff"
							>{positionHeader}</span
						></th
					>
					<th class="w-20 pr-4 pb-2 text-left"><span class="eyebrow">Nation</span></th>
					<th class="w-14 pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('age')}>
							Age{scout.sortKey === 'age' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-20 pr-4 pb-2 text-right"><span class="eyebrow">Wage</span></th>
					<th class="w-20 pr-4 pb-2 text-right">
						<button
							class="eyebrow hover:text-[var(--color-mist)]"
							title="Minimum fee release clause, from the save's own contract data. Blank is no clause decoded, not a free transfer."
							onclick={() => scout.sortBy('clause')}
						>
							Clause{scout.sortKey === 'clause'
								? scout.sortDirection === 'asc'
									? ' ↑'
									: ' ↓'
								: ''}
						</button>
					</th>
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
					{#if showReputation}
						<th class="w-16 pr-4 pb-2 text-left">
							<button
								class="eyebrow hover:text-[var(--color-mist)]"
								title="Worldwide reputation, 0-200 — the one that decides who will take your call. For staff it's the non-player sheet; for a player it's bound only where it repeats their own CA/PA, so coverage runs high but is not total. Home and current reputation are in the detail panel."
								onclick={() => scout.sortBy('reputation')}
							>
								Rep{scout.sortKey === 'reputation'
									? scout.sortDirection === 'asc'
										? ' ↑'
										: ' ↓'
									: ''}
							</button>
						</th>
					{/if}
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
					<PlayerRow
						{player}
						{showReputation}
						score={profile ? (scout.scores.get(player.id) ?? null) : undefined}
					/>
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
