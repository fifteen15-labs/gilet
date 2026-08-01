<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';

	const total = $derived(scout.matchingClubs().length);
	const rows = $derived(scout.visibleClubs());
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="min-h-0 flex-1 overflow-y-auto">
		<table class="w-full border-collapse">
			<thead class="sticky top-0 z-10 bg-[var(--color-void)]">
				<tr class="border-b border-[var(--color-line)]">
					<th class="pr-4 pb-2 pl-3 text-left">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => (scout.clubSort = 'name')}>
							Club{scout.clubSort === 'name' ? ' ↑' : ''}
						</button>
					</th>
					<th class="w-48 pr-4 pb-2 text-left"><span class="eyebrow">Short name</span></th>
					<th class="w-20 pr-4 pb-2 text-right"><span class="eyebrow">Squad</span></th>
					<th class="w-20 pr-4 pb-2 text-right">
						<button class="eyebrow hover:text-[var(--color-mist)]" onclick={() => (scout.clubSort = 'strength')}>
							Avg CA{scout.clubSort === 'strength' ? ' ↓' : ''}
						</button>
					</th>
					<th class="w-20 pr-3 pb-2 text-right"><span class="eyebrow">Avg PA</span></th>
				</tr>
			</thead>
			<tbody>
				{#each rows as club (club.id)}
					<tr
						class="cursor-pointer border-b border-[var(--color-line-soft)] hover:bg-[var(--color-panel)]
							{scout.selectedId === club.id ? 'bg-[var(--color-raised)]' : ''}"
						onclick={() => (scout.selectedId = club.id)}
					>
						<td class="py-1.5 pr-4 pl-3 text-sm text-[var(--color-bright)]">{club.name}</td>
						<td class="pr-4 text-sm text-[var(--color-mist)]">{club.short_name}</td>
						<td class="tabular pr-4 text-right text-xs text-[var(--color-faint)]">
							{club.squad_size || ''}
						</td>
						<td class="tabular pr-4 text-right text-sm text-[var(--color-bright)]">
							{club.average_ability ?? ''}
						</td>
						<td class="tabular pr-3 text-right text-sm text-[var(--color-signal)]">
							{club.average_potential ?? ''}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>

		{#if total === 0}
			<p class="px-4 py-8 text-sm text-[var(--color-faint)]">
				No clubs match that search. Try the short name, like "Man City".
			</p>
		{/if}
	</div>

	<footer class="border-t border-[var(--color-line)] px-4 py-2 text-xs text-[var(--color-faint)]">
		<span class="tabular">
			{#if total > rows.length}
				Showing {rows.length.toLocaleString()} of {total.toLocaleString()} — search to narrow
			{:else}
				{total.toLocaleString()}
				{total === 1 ? 'club' : 'clubs'}
			{/if}
		</span>
	</footer>
</div>
