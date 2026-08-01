<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import type { Player } from '$lib/tauri/commands';

	type Props = {
		player: Player;
		shortlisted: boolean;
		onToggle: (name: string) => void;
	};

	const { player, shortlisted, onToggle }: Props = $props();
</script>

<!-- The row opens the detail panel; the checkbox stops propagation so ticking
	does not also select. -->
<tr
	class="group cursor-pointer border-b border-[var(--color-line-soft)] hover:bg-[var(--color-panel)]
		{scout.selectedId === player.id ? 'bg-[var(--color-raised)]' : ''}"
	onclick={() => (scout.selectedId = player.id)}
>
	<td class="w-8 pl-3">
		<button
			type="button"
			class="flex h-5 w-5 items-center justify-center rounded-[2px] border transition-colors
				{shortlisted
				? 'border-[var(--color-hivis)] bg-[var(--color-hivis)] text-[var(--color-void)]'
				: 'border-[var(--color-line)] text-transparent hover:border-[var(--color-faint)]'}"
			aria-pressed={shortlisted}
			aria-label={shortlisted ? `Remove ${player.name} from shortlist` : `Add ${player.name} to shortlist`}
			onclick={(event) => {
				event.stopPropagation();
				onToggle(player.name);
			}}
		>
			<svg viewBox="0 0 10 10" class="h-2.5 w-2.5" aria-hidden="true">
				<path d="M1 5l2.5 2.5L9 2" fill="none" stroke="currentColor" stroke-width="2" />
			</svg>
		</button>
	</td>
	<td class="py-1.5 pr-4 text-sm text-[var(--color-bright)]">{player.name}</td>
	<td class="tabular pr-4 text-sm text-[var(--color-mist)]">{player.age}</td>
	<td class="tabular pr-4 text-sm text-[var(--color-bright)]">{player.ability ?? ''}</td>
	<td class="tabular pr-4 text-sm text-[var(--color-signal)]">{player.potential ?? ''}</td>
	<td class="pr-3">
		<AbilityBar ability={player.ability} potential={player.potential} />
	</td>
</tr>
