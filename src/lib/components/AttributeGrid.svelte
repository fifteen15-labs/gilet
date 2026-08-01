<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';

	type Props = { attributes: number[] };
	const { attributes }: Props = $props();

	const gk = $derived(new Set(scout.summary?.goalkeeping_indices ?? []));

	type Entry = { index: number; value: number };
	const outfield = $derived(
		attributes.map((value, index) => ({ index, value })).filter((e) => !gk.has(e.index))
	);
	const goalkeeping = $derived(
		attributes.map((value, index) => ({ index, value })).filter((e) => gk.has(e.index))
	);

	/** FM's own colouring: weak values recede, strong ones stand out. */
	function tone(value: number): string {
		if (value >= 15) return 'text-[var(--color-signal)]';
		if (value >= 10) return 'text-[var(--color-bright)]';
		if (value >= 6) return 'text-[var(--color-mist)]';
		return 'text-[var(--color-faint)]';
	}

	function label(entry: Entry): string {
		return `Attribute ${entry.index + 1}`;
	}
</script>

{#snippet group(title: string, entries: Entry[])}
	{#if entries.length > 0}
		<div class="mb-4">
			<h4 class="eyebrow mb-1.5">{title}</h4>
			<div class="grid grid-cols-2 gap-x-4 gap-y-0.5">
				{#each entries as entry (entry.index)}
					<div class="flex items-baseline justify-between border-b border-[var(--color-line-soft)] py-0.5">
						<span class="text-xs text-[var(--color-faint)]">{label(entry)}</span>
						<span class="tabular text-xs {tone(entry.value)}">{entry.value}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}
{/snippet}

{#if attributes.length === 0}
	<p class="text-xs leading-relaxed text-[var(--color-faint)]">
		No attributes. Staff records carry no attribute block, which is how players and staff are told
		apart.
	</p>
{:else}
	{@render group('Outfield', outfield)}
	{@render group('Goalkeeping', goalkeeping)}
	<p class="text-xs leading-relaxed text-[var(--color-faint)]">
		Values are on FM's 1&ndash;20 scale. The goalkeeping set is identified from the data; individual
		attribute names are not mapped yet, so each is shown by its position.
	</p>
{/if}
