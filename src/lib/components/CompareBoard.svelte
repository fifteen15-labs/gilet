<script lang="ts">
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { DIRTINESS, INJURY_PRONENESS } from '$lib/utils/attributes';
	import { flagsFor, headroom } from '$lib/utils/flags';
	import { coverage } from '$lib/utils/positions';

	const players = $derived(scout.compared);
	const names = $derived(scout.summary?.attribute_names ?? []);
	const gk = $derived(new Set(scout.summary?.goalkeeping_indices ?? []));
	const profile = $derived(profiles.active);

	/** FM's five hidden attributes, kept in their own group the way the detail
	 * panel does — a scout reads them differently from the visible set. */
	const HIDDEN = new Set([41, 44, 47, 48, 49]);

	/**
	 * The two attributes where a high number is bad news. Everything else on
	 * the 1-20 scale reads "more is better", so the leader mark would point at
	 * the wrong player on exactly these two without saying so.
	 */
	const LOWER_IS_BETTER = new Set([DIRTINESS, INJURY_PRONENESS]);

	type Row = { index: number; label: string };

	function group(keep: (index: number) => boolean): Row[] {
		return names
			.map((label, index) => ({ label, index }))
			.filter((row) => row.label !== '' && keep(row.index));
	}

	const outfield = $derived(group((i) => !gk.has(i) && !HIDDEN.has(i)));
	const goalkeeping = $derived(group((i) => gk.has(i)));
	const hidden = $derived(group((i) => HIDDEN.has(i)));

	/**
	 * Which columns hold the best value in a row, as a set of column indices.
	 *
	 * Empty when every value matches or when fewer than two players have one:
	 * marking a "winner" among identical numbers, or against a player whose
	 * value is simply undecoded, would be reading a result out of nothing.
	 */
	function leaders(values: (number | null)[], lowerIsBetter = false): Set<number> {
		const known = values.filter((v): v is number => v !== null);
		if (known.length < 2) return new Set();
		const best = lowerIsBetter ? Math.min(...known) : Math.max(...known);
		if (best === (lowerIsBetter ? Math.max(...known) : Math.min(...known))) return new Set();
		const found = new Set<number>();
		values.forEach((v, column) => {
			if (v === best) found.add(column);
		});
		return found;
	}

	function attributeRow(index: number): (number | null)[] {
		return players.map((p) => p.attributes[index] ?? null);
	}

	/** Compact weekly wage, matching the table's own formatting. */
	function formatWage(wage: number | null): string {
		if (wage === null) return '—';
		if (wage >= 100_000) return `£${Math.round(wage / 1000)}K`;
		if (wage >= 1_000) return `£${(wage / 1000).toFixed(1).replace(/\.0$/, '')}K`;
		return `£${wage}`;
	}

	const summaryRows = $derived([
		{ label: 'Age', values: players.map((p) => p.age), lower: false, mark: false },
		{ label: 'Current ability', values: players.map((p) => p.ability), lower: false, mark: true },
		{ label: 'Max ability', values: players.map((p) => p.potential), lower: false, mark: true },
		{ label: 'Room to grow', values: players.map(headroom), lower: false, mark: true },
		{
			label: 'Positions covered',
			values: players.map((p) => coverage(p.position_ratings)),
			lower: false,
			mark: true
		}
	]);
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="flex items-center gap-3 border-b border-[var(--color-line)] px-4 py-2.5">
		<h2 class="eyebrow">Comparing {players.length}</h2>
		<p class="text-xs text-[var(--color-faint)]">
			Highlighted is the best of those on the board, not a verdict on the player.
		</p>
		<div class="ml-auto flex items-center gap-2">
			<button
				type="button"
				class="text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
				onclick={() => scout.clearPinned()}
			>
				Clear board
			</button>
			<button
				type="button"
				class="text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
				onclick={() => (scout.comparing = false)}
			>
				Back to table
			</button>
		</div>
	</div>

	<div class="min-h-0 flex-1 overflow-auto px-4 py-3">
		<table class="w-full border-collapse">
			<thead class="sticky top-0 z-10 bg-[var(--color-void)]">
				<tr class="border-b border-[var(--color-line)]">
					<th class="w-44 pr-4 pb-2 text-left"><span class="eyebrow">Attribute</span></th>
					{#each players as player (player.id)}
						<th class="pr-4 pb-2 text-left align-bottom">
							<div class="flex items-start justify-between gap-2">
								<div class="min-w-0">
									<div class="truncate text-sm text-[var(--color-bright)]">{player.name}</div>
									<div class="truncate text-xs text-[var(--color-faint)]">
										{player.club || '—'} · {formatWage(player.wage)}
									</div>
								</div>
								<button
									type="button"
									class="shrink-0 text-xs text-[var(--color-faint)] hover:text-[var(--color-hivis)]"
									aria-label="Remove {player.name} from the board"
									onclick={() => scout.togglePinned(player.id)}>×</button
								>
							</div>
						</th>
					{/each}
				</tr>
			</thead>

			<tbody>
				{#each summaryRows as row (row.label)}
					{@const best = row.mark ? leaders(row.values, row.lower) : new Set()}
					<tr class="border-b border-[var(--color-line-soft)]">
						<td class="py-1 pr-4 text-xs text-[var(--color-mist)]">{row.label}</td>
						{#each row.values as value, column (column)}
							<td
								class="tabular py-1 pr-4 text-sm
									{best.has(column) ? 'text-[var(--color-signal)]' : 'text-[var(--color-bright)]'}"
							>
								{value ?? '—'}
							</td>
						{/each}
					</tr>
				{/each}

				{#if profile}
					{@const scores = players.map((p) => scout.scores.get(p.id) ?? null)}
					{@const best = leaders(scores)}
					<tr class="border-b border-[var(--color-line-soft)]">
						<td class="py-1 pr-4 text-xs text-[var(--color-mist)]" title="Your weights, not an FM figure">
							{profile.name}
						</td>
						{#each scores as value, column (column)}
							<td
								class="tabular py-1 pr-4 text-sm
									{best.has(column) ? 'text-[var(--color-signal)]' : 'text-[var(--color-bright)]'}"
							>
								{value ?? '—'}
							</td>
						{/each}
					</tr>
				{/if}

				<tr>
					<td class="pt-3 pr-4 text-xs" colspan={players.length + 1}>
						<span class="eyebrow">Flags</span>
					</td>
				</tr>
				<tr class="border-b border-[var(--color-line)]">
					<td class="pb-2"></td>
					{#each players as player (player.id)}
						<td class="pr-4 pb-2 align-top">
							<div class="flex flex-wrap gap-1">
								{#each flagsFor(player) as flag (flag.key)}
									<span
										class="rounded-[2px] border px-1 py-0.5 text-[10px]
											{flag.tone === 'risk'
											? 'border-[var(--color-hivis-dim)] text-[var(--color-hivis)]'
											: 'border-[var(--color-signal-dim)] text-[var(--color-signal)]'}"
										title="{flag.label} {flag.value}"
									>
										{flag.label}
									</span>
								{:else}
									<span class="text-xs text-[var(--color-faint)]">—</span>
								{/each}
							</div>
						</td>
					{/each}
				</tr>

				{#each [{ title: 'Outfield', rows: outfield }, { title: 'Goalkeeping', rows: goalkeeping }, { title: 'Hidden', rows: hidden }] as section (section.title)}
					{#if section.rows.length > 0}
						<tr>
							<td class="pt-3 pb-1" colspan={players.length + 1}>
								<span class="eyebrow">{section.title}</span>
							</td>
						</tr>
						{#each section.rows as row (row.index)}
							{@const values = attributeRow(row.index)}
							{@const best = leaders(values, LOWER_IS_BETTER.has(row.index))}
							<tr class="border-b border-[var(--color-line-soft)]">
								<td class="py-0.5 pr-4 text-xs text-[var(--color-mist)]">
									{row.label}{LOWER_IS_BETTER.has(row.index) ? ' ↓' : ''}
								</td>
								{#each values as value, column (column)}
									<td
										class="tabular py-0.5 pr-4 text-xs
											{best.has(column) ? 'text-[var(--color-signal)]' : 'text-[var(--color-bright)]'}"
									>
										{value ?? '—'}
									</td>
								{/each}
							</tr>
						{/each}
					{/if}
				{/each}
			</tbody>
		</table>

		<p class="mt-3 text-xs leading-relaxed text-[var(--color-faint)]">
			A ↓ marks the two attributes where less is better, so the highlight points at the right
			player. Rows where everyone matches, or where only one player has a decoded value, are left
			unmarked rather than declaring a winner out of nothing.
		</p>
	</div>
</div>
