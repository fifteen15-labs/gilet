<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { abilityOf, flagsFor, headroom, potentialOf } from '$lib/utils/flags';
	import { formatWage } from '$lib/utils/money';
	import type { Player } from '$lib/tauri/commands';

	type Props = {
		player: Player;
		/** Score under the active profile; null when this person cannot be
		 * scored, undefined when no profile is active and the column is absent. */
		score?: number | null;
		/** Whether the table is showing its world-reputation column, which it
		 * only does under the Staff kind. */
		showReputation?: boolean;
	};

	const { player, score, showReputation = false }: Props = $props();

	const room = $derived(headroom(player));
	/** A player's own ability, or the staff sheet's non-player CA/PA — same
	 * 0-200 scale, both the save's own numbers. */
	const current = $derived(abilityOf(player));
	const potential = $derived(potentialOf(player));
	const flags = $derived(flagsFor(player));
	const risks = $derived(flags.filter((f) => f.tone === 'risk'));
	const strengths = $derived(flags.filter((f) => f.tone === 'strength'));
	/** The whole report in one hover, so the table stays a table. */
	const flagTitle = $derived(flags.map((f) => `${f.label} (${f.value})`).join('\n'));
</script>

<!-- The row opens the detail panel, where the in-save shortlist actions live. -->
<tr
	class="group cursor-pointer border-b border-[var(--color-line-soft)] hover:bg-[var(--color-panel)]
		{scout.selectedId === player.id ? 'bg-[var(--color-raised)]' : ''}
		{scout.isPinned(player.id) ? 'border-l-2 border-l-[var(--color-hivis)]' : ''}"
	onclick={() => scout.select(player)}
>
	<td class="py-1.5 pr-4 pl-3 text-sm text-[var(--color-bright)]">
		{#if player.stub}
			<span class="text-[var(--color-faint)] italic">Unnamed — non-contract</span>
		{:else}
			{player.name}
		{/if}
	</td>
	<td class="pr-4 text-xs text-[var(--color-mist)]">{player.club}</td>
	<td class="pr-4 text-xs text-[var(--color-mist)]">
		{player.positions.length > 0
			? player.positions.slice(0, 3).join(', ')
			: (player.staff_role ?? '')}
	</td>
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
	<td
		class="tabular pr-4 text-sm text-[var(--color-bright)]"
		title={player.is_player || current === null ? '' : 'Non-player ability'}
	>
		{current ?? ''}
	</td>
	<td
		class="tabular pr-4 text-sm text-[var(--color-signal)]"
		title={player.is_player || potential === null ? '' : 'Non-player potential'}
	>
		{potential ?? ''}
	</td>
	<td class="tabular pr-4 text-sm text-[var(--color-mist)]">{room === null ? '' : `+${room}`}</td>
	{#if showReputation}
		<td
			class="tabular pr-4 text-sm text-[var(--color-mist)]"
			title={player.staff
				? `Home ${player.staff.homeReputation} · current ${player.staff.currentReputation} · world ${player.staff.worldReputation}`
				: ''}
		>
			{player.staff?.worldReputation ?? ''}
		</td>
	{/if}
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
		<AbilityBar ability={current} potential={potential} />
	</td>
</tr>
