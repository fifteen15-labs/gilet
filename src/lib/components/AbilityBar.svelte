<script lang="ts">
	/**
	 * The signature element: current ability as a solid fill, the headroom up to
	 * potential as a lighter extension. The gap between the two is the whole
	 * point of scouting, so it should be readable at a glance down a column.
	 *
	 * When ability has not been decoded from the save, the track renders hatched
	 * rather than empty — an instrument showing no reading, not a zero.
	 */
	type Props = {
		ability: number | null;
		potential: number | null;
	};

	const { ability, potential }: Props = $props();

	// Current and Potential Ability are on a 1-200 scale in FM's own data.
	const SCALE = 200;

	const known = $derived(ability !== null);
	const abilityPct = $derived(ability === null ? 0 : Math.min(100, (ability / SCALE) * 100));
	const potentialPct = $derived(
		potential === null ? abilityPct : Math.min(100, (potential / SCALE) * 100)
	);
</script>

{#if known}
	<div class="flex items-center gap-2">
		<div
			class="relative h-1.5 w-28 overflow-hidden rounded-[1px] bg-[var(--color-line-soft)]"
			role="img"
			aria-label="Ability {ability} of a possible {potential ?? ability}"
		>
			<!-- Headroom sits behind the fill so the two read as one measurement. -->
			<div
				class="absolute inset-y-0 left-0 bg-[var(--color-signal-dim)]"
				style="width: {potentialPct}%"
			></div>
			<div class="absolute inset-y-0 left-0 bg-[var(--color-signal)]" style="width: {abilityPct}%"></div>
		</div>
		<span class="tabular text-xs text-[var(--color-bright)]">{ability}</span>
		{#if potential !== null && potential > (ability ?? 0)}
			<span class="tabular text-xs text-[var(--color-faint)]">/{potential}</span>
		{/if}
	</div>
{:else}
	<div class="flex items-center gap-2" title="Ability data is not decoded from the save format yet">
		<div class="hatched h-1.5 w-28 rounded-[1px]" role="img" aria-label="Ability not available"></div>
		<span class="eyebrow">No reading</span>
	</div>
{/if}
