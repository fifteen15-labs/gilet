<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { auditSquad } from '$lib/utils/audit';
	import type { Club } from '$lib/tauri/commands';

	type Props = { club: Club };
	const { club }: Props = $props();

	/** A year past the save's own date. Null when the save could not say what
	 * day it is, in which case nothing is called expiring — the system clock is
	 * not this save's calendar. */
	const expiryCutoff = $derived.by(() => {
		const base = scout.summary?.game_date;
		if (!base) return null;
		return `${Number(base.slice(0, 4)) + 1}${base.slice(4)}`;
	});

	const audit = $derived(auditSquad(club, scout.players, expiryCutoff));
	/** The widest position count, so the bars are relative to this squad rather
	 * than to some absolute idea of a full one. */
	const busiest = $derived(Math.max(1, ...audit.positions.map((p) => p.count)));

	/** Jumps to the people table showing this squad, with a filter applied. */
	function drill(patch: Parameters<typeof scout.screen>[0]) {
		scout.screen({ query: club.short_name, ...patch }, 'age');
	}
</script>

{#if audit.counted === 0}
	<p class="mt-4 text-xs leading-relaxed text-[var(--color-faint)]">
		No squad to audit. This club's players are outside the loaded leagues, so the save never
		materialised their squad list.
	</p>
{:else}
	<div class="mt-5">
		<h4 class="eyebrow mb-2">Squad audit</h4>

		<dl class="mb-3 grid grid-cols-2 gap-x-4 gap-y-2">
			<div>
				<dt class="eyebrow">Average age</dt>
				<dd class="tabular text-sm text-[var(--color-bright)]">{audit.averageAge ?? '—'}</dd>
			</div>
			<div>
				<dt class="eyebrow" title="Mean of max ability minus current, across the squad">
					Room to grow
				</dt>
				<dd class="tabular text-sm text-[var(--color-signal)]">
					{audit.averageHeadroom === null ? '—' : `+${audit.averageHeadroom}`}
				</dd>
			</div>
		</dl>

		<!-- Age shape: the three bands a squad is actually managed by. -->
		<div class="mb-3">
			<div class="mb-1 flex h-1.5 w-full overflow-hidden rounded-[1px] bg-[var(--color-raised)]">
				{#each [{ n: audit.ages.young, c: 'var(--color-signal)' }, { n: audit.ages.prime, c: 'var(--color-mist)' }, { n: audit.ages.older, c: 'var(--color-hivis-dim)' }] as band (band.c)}
					{#if band.n > 0}
						<div style="width: {(band.n / audit.counted) * 100}%; background: {band.c}"></div>
					{/if}
				{/each}
			</div>
			<div class="flex justify-between text-xs text-[var(--color-faint)]">
				<span class="tabular">U21 {audit.ages.young}</span>
				<span class="tabular">22–28 {audit.ages.prime}</span>
				<span class="tabular">29+ {audit.ages.older}</span>
			</div>
		</div>

		<h4 class="eyebrow mt-4 mb-1.5">Cover by position</h4>
		<div class="mb-1 space-y-0.5">
			{#each audit.positions as slot (slot.code)}
				<div class="flex items-center gap-2">
					<span class="w-9 shrink-0 text-xs text-[var(--color-mist)]">{slot.code}</span>
					<div class="h-1.5 flex-1 rounded-[1px] bg-[var(--color-raised)]">
						{#if slot.count > 0}
							<div
								class="h-full rounded-[1px] bg-[var(--color-signal)]"
								style="width: {(slot.count / busiest) * 100}%"
							></div>
						{/if}
					</div>
					<span
						class="tabular w-4 shrink-0 text-right text-xs
							{slot.count === 0 ? 'text-[var(--color-hivis)]' : 'text-[var(--color-faint)]'}"
					>
						{slot.count}
					</span>
				</div>
			{/each}
		</div>
		<p class="mb-3 text-xs leading-relaxed text-[var(--color-faint)]">
			{#if audit.gaps.length > 0}
				Nobody plays {audit.gaps.join(', ')}.
			{:else}
				Every position has someone who plays there.
			{/if}
			A player counts under every position they are comfortable in, so the bars total more than
			the squad.
		</p>

		<h4 class="eyebrow mt-4 mb-1.5">Worth a look</h4>
		<div class="space-y-1">
			<button
				type="button"
				class="flex w-full items-center justify-between rounded-[2px] border border-[var(--color-line)] px-2 py-1
					text-xs text-[var(--color-mist)] transition-colors hover:border-[var(--color-hivis)]
					disabled:cursor-not-allowed disabled:opacity-40"
				disabled={audit.expiring === 0}
				title={expiryCutoff === null
					? "This save's own date could not be read, so nothing can be called expiring"
					: `Contracts ending on or before ${expiryCutoff}`}
				onclick={() => drill({ contract: 'expiring', expiryCutoff })}
			>
				<span>Contracts running down</span>
				<span class="tabular">{audit.expiring}</span>
			</button>
			<button
				type="button"
				class="flex w-full items-center justify-between rounded-[2px] border border-[var(--color-line)] px-2 py-1
					text-xs text-[var(--color-mist)] transition-colors hover:border-[var(--color-hivis)]
					disabled:cursor-not-allowed disabled:opacity-40"
				disabled={audit.flagged === 0}
				title="Players carrying at least one red flag — injury proneness, temperament, professionalism"
				onclick={() => drill({ risk: 'flagged' })}
			>
				<span>Carrying a red flag</span>
				<span class="tabular">{audit.flagged}</span>
			</button>
		</div>

		{#if audit.unreadable > 0 || audit.undated > 0}
			<p class="mt-3 text-xs leading-relaxed text-[var(--color-faint)]">
				{#if audit.unreadable > 0}
					{audit.unreadable} of the club's {club.squad_size} squad places could not be resolved to a
					person, so they are missing from every count above rather than assumed away.
				{/if}
				{#if audit.undated > 0}
					{audit.undated}
					{audit.undated === 1 ? 'player has' : 'players have'} no contract date decoded — unknown, not
					expiring.
				{/if}
			</p>
		{/if}
	</div>
{/if}
