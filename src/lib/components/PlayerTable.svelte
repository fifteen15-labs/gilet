<script lang="ts">
	import PlayerRow from './PlayerRow.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';

	const members = $derived(shortlists.activeMembers);
	const total = $derived(scout.matching(members).length);
	const rows = $derived(scout.visible(members));

	function toggle(name: string) {
		void shortlists.toggle(name);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="min-h-0 flex-1 overflow-y-auto">
		<table class="w-full border-collapse">
			<thead class="sticky top-0 z-10 bg-[var(--color-void)]">
				<tr class="border-b border-[var(--color-line)]">
					<th class="w-8"></th>
					<th class="pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('name')}>
							Name{scout.sortKey === 'name' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
						</button>
					</th>
					<th class="w-14 pr-4 pb-2 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => scout.sortBy('age')}>
							Age{scout.sortKey === 'age' ? (scout.sortDirection === 'asc' ? ' ↑' : ' ↓') : ''}
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
					<th class="w-40 pr-3 pb-2 text-left"><span class="eyebrow">Ability / max</span></th>
				</tr>
			</thead>
			<tbody>
				{#each rows as player (player.id)}
					<PlayerRow {player} shortlisted={members.has(player.name)} onToggle={toggle} />
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
		{#if shortlists.active}
			<span class="tabular">
				{shortlists.active.name}: {shortlists.active.players.length}
			</span>
		{/if}
	</footer>
</div>
