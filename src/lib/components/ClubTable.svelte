<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { formatBill } from '$lib/utils/money';

	const total = $derived(scout.matchingClubs().length);
	const rows = $derived(scout.visibleClubs());

	/** What a club's wage bill is missing, said out loud on hover. The sum is
	 * only the wages that decoded, so a club with players the parser could not
	 * read a contract for has a floor, not a bill. */
	function billTitle(known: number, squad: number): string {
		if (squad === 0) return '';
		if (known >= squad) return `All ${squad} squad wages decoded. Staff wages are not decoded at all.`;
		return `${known} of ${squad} squad wages decoded — this is a floor, not the club's outgoings. Staff wages are not decoded at all.`;
	}
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
					<th class="w-20 pr-4 pb-2 text-right"><span class="eyebrow">Avg PA</span></th>
					<th class="w-20 pr-4 pb-2 text-right">
						<button
							class="eyebrow hover:text-[var(--color-mist)]"
							title="Mean age of the squad players whose birth date decoded, on the save's own date. Sorts youngest first — the end of the column worth looking at. Staff are not in it."
							onclick={() => (scout.clubSort = 'age')}
						>
							Avg age{scout.clubSort === 'age' ? ' ↑' : ''}
						</button>
					</th>
					<th class="w-24 pr-3 pb-2 text-right">
						<button
							class="eyebrow hover:text-[var(--color-mist)]"
							title="Weekly wage bill: the sum of the squad wages that decoded. Hover a club to see how many of its squad that covers — staff wages aren't decoded at all, so this is the playing bill."
							onclick={() => (scout.clubSort = 'wages')}
						>
							Wages/w{scout.clubSort === 'wages' ? ' ↓' : ''}
						</button>
					</th>
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
						<td class="tabular pr-4 text-right text-sm text-[var(--color-signal)]">
							{club.average_potential ?? ''}
						</td>
						<td class="tabular pr-4 text-right text-sm text-[var(--color-mist)]">
							{club.average_age ?? ''}
						</td>
						<td
							class="tabular pr-3 text-right text-sm text-[var(--color-mist)]"
							title={billTitle(club.wages_known, club.squad_size)}
						>
							{formatBill(club.wage_bill)}
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
