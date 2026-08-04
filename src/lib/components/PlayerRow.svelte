<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { flagsFor, headroom } from '$lib/utils/flags';
	import type { Player } from '$lib/tauri/commands';

	type Props = {
		player: Player;
		/** Score under the active profile; null when this person cannot be
		 * scored, undefined when no profile is active and the column is absent. */
		score?: number | null;
	};

	const { player, score }: Props = $props();

	const room = $derived(headroom(player));
	const flags = $derived(flagsFor(player));
	const risks = $derived(flags.filter((f) => f.tone === 'risk'));
	const strengths = $derived(flags.filter((f) => f.tone === 'strength'));
	/** The whole report in one hover, so the table stays a table. */
	const flagTitle = $derived(flags.map((f) => `${f.label} (${f.value})`).join('\n'));

	/** Compact weekly wage: £450K, £8.5K, £400 — or nothing when out of contract. */
	function formatWage(wage: number | null): string {
		if (wage === null) return '';
		if (wage >= 100_000) return `£${Math.round(wage / 1000)}K`;
		if (wage >= 1_000) return `£${(wage / 1000).toFixed(1).replace(/\.0$/, '')}K`;
		return `£${wage}`;
	}
</script>

<!-- The row opens the detail panel, where the in-save shortlist actions live. -->
<tr
	class="group cursor-pointer border-b border-[var(--color-line-soft)] hover:bg-[var(--color-panel)]
		{scout.selectedId === player.id ? 'bg-[var(--color-raised)]' : ''}
		{scout.isPinned(player.id) ? 'border-l-2 border-l-[var(--color-hivis)]' : ''}"
	onclick={() => (scout.selectedId = player.id)}
>
	<td class="py-1.5 pr-4 pl-3 text-sm text-[var(--color-bright)]">
		{#if player.stub}
			<span class="text-[var(--color-faint)] italic">Unnamed — non-contract</span>
		{:else}
			{player.name}
		{/if}
	</td>
	<td class="pr-4 text-xs text-[var(--color-mist)]">{player.club}</td>
	<td class="pr-4 text-xs text-[var(--color-mist)]">{player.positions.slice(0, 3).join(', ')}</td>
	<td
		class="pr-4 text-xs text-[var(--color-faint)]"
		title={player.nation_id === null ? '' : `Nation ${player.nation_id}`}
	>
		{player.nation || (player.nation_id ?? '')}
	</td>
	<td class="tabular pr-4 text-sm text-[var(--color-mist)]">{player.age ?? ''}</td>
	<td class="tabular pr-4 text-right text-xs text-[var(--color-mist)]" title={player.contract_until ? `Contract until ${player.contract_until}` : ''}>
		{formatWage(player.wage)}
	</td>
	<td class="tabular pr-4 text-sm text-[var(--color-bright)]">{player.ability ?? ''}</td>
	<td class="tabular pr-4 text-sm text-[var(--color-signal)]">{player.potential ?? ''}</td>
	<td class="tabular pr-4 text-sm text-[var(--color-mist)]">{room === null ? '' : `+${room}`}</td>
	{#if score !== undefined}
		<td class="tabular pr-4 text-sm text-[var(--color-bright)]">{score ?? ''}</td>
	{/if}
	<td class="tabular pr-4 text-xs" title={flagTitle}>
		{#if risks.length > 0}
			<span class="text-[var(--color-hivis)]">▲{risks.length}</span>
		{/if}
		{#if strengths.length > 0}
			<span class="ml-1 text-[var(--color-signal)]">★{strengths.length}</span>
		{/if}
	</td>
	<td class="pr-3">
		<AbilityBar ability={player.ability} potential={player.potential} />
	</td>
</tr>
