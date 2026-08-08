<script lang="ts">
	/**
	 * Up to four pinned people, side by side over every figure they share.
	 *
	 * The board reads whichever sheet its people carry: the 54 player
	 * attributes, or the 52-item non-player one. Pin a player and a coach
	 * together and it falls back to the figures both actually have, because
	 * laying one sheet's labels over the other's numbers would compare nothing.
	 */
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { staffAttributeNames } from '$lib/tauri/commands';
	import { DIRTINESS, INJURY_PRONENESS, STAFF_COACHING_FROM } from '$lib/utils/attributes';
	import { boardMode, leaders, staffTendenciesDecoded } from '$lib/utils/compare';
	import { abilityOf, flagsFor, headroom, potentialOf } from '$lib/utils/flags';
	import { formatWage } from '$lib/utils/money';
	import { coverage } from '$lib/utils/positions';
	import { score } from '$lib/utils/score';

	const players = $derived(scout.compared);
	const names = $derived(scout.summary?.attribute_names ?? []);
	const gk = $derived(new Set(scout.summary?.goalkeeping_indices ?? []));
	const profile = $derived(profiles.active);
	const mode = $derived(boardMode(players));

	/** The same 52 labels for every person in the save, fetched once. */
	let staffNames = $state<string[]>([]);
	$effect(() => {
		staffAttributeNames().then((n) => (staffNames = n));
	});

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

	function group(labels: readonly string[], keep: (index: number) => boolean): Row[] {
		return labels
			.map((label, index) => ({ label, index }))
			.filter((row) => row.label !== '' && keep(row.index));
	}

	const outfield = $derived(group(names, (i) => !gk.has(i) && !HIDDEN.has(i)));
	const goalkeeping = $derived(group(names, (i) => gk.has(i)));
	const hidden = $derived(group(names, (i) => HIDDEN.has(i)));

	/** The staff sheet in the editor's own two halves. The tendency half is
	 * only offered when every sheet on the board still reads on the editor's
	 * 1-20 scale; an aged save moves it somewhere nobody has decoded. */
	const coaching = $derived(group(staffNames, (i) => i >= STAFF_COACHING_FROM));
	const tendencies = $derived(group(staffNames, (i) => i < STAFF_COACHING_FROM));
	const tendenciesDecoded = $derived(staffTendenciesDecoded(players));

	function attributeRow(index: number): (number | null)[] {
		return players.map((p) =>
			mode === 'staff' ? (p.staff?.attributes[index] ?? null) : (p.attributes[index] ?? null)
		);
	}

	/** The rows both sheets share. Ability reads a player's own block or the
	 * non-player CA/PA, the same resolution the table uses, so a staff board is
	 * not a column of dashes. */
	const summaryRows = $derived([
		{ label: 'Age', values: players.map((p) => p.age), lower: false, mark: false },
		{ label: 'Current ability', values: players.map(abilityOf), lower: false, mark: true },
		{ label: 'Max ability', values: players.map(potentialOf), lower: false, mark: true },
		{ label: 'Room to grow', values: players.map(headroom), lower: false, mark: true },
		...(mode === 'staff'
			? [
					{
						label: 'Reputation (world)',
						values: players.map((p) => p.staff?.worldReputation ?? null),
						lower: false,
						mark: true
					},
					{
						label: 'Reputation (current)',
						values: players.map((p) => p.staff?.currentReputation ?? null),
						lower: false,
						mark: true
					}
				]
			: [
					{
						label: 'Positions covered',
						values: players.map((p) => coverage(p.position_ratings)),
						lower: false,
						mark: true
					}
				])
	]);

	/** Which attribute sections the board draws, by mode. Mixed draws none. */
	const sections = $derived(
		mode === 'player'
			? [
					{ title: 'Outfield', rows: outfield },
					{ title: 'Goalkeeping', rows: goalkeeping },
					{ title: 'Hidden', rows: hidden }
				]
			: mode === 'staff'
				? [
						{ title: 'Coaching and knowledge', rows: coaching },
						...(tendenciesDecoded ? [{ title: 'Tendencies', rows: tendencies }] : [])
					]
				: []
	);
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
										{player.club || '—'} · {player.staff_role ?? formatWage(player.wage, '—')}
									</div>
								</div>
								<button
									type="button"
									class="shrink-0 text-xs text-[var(--color-faint)] hover:text-[var(--color-hivis)]"
									aria-label="Remove {player.name} from the board"
									onclick={() => scout.togglePinned(player)}>×</button
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
					<!-- Scored here rather than looked up: the backend scores the
						table's page, and a pinned player may be off it. -->
					{@const scores = players.map((p) => score(p, profile))}
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

				{#if mode === 'player'}
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
				{/if}

				{#each sections as section (section.title)}
					{#if section.rows.length > 0}
						<tr>
							<td class="pt-3 pb-1" colspan={players.length + 1}>
								<span class="eyebrow">{section.title}</span>
							</td>
						</tr>
						{#each section.rows as row (row.index)}
							{@const values = attributeRow(row.index)}
							{@const best = leaders(values, mode === 'player' && LOWER_IS_BETTER.has(row.index))}
							<tr class="border-b border-[var(--color-line-soft)]">
								<td class="py-0.5 pr-4 text-xs text-[var(--color-mist)]">
									{row.label}{mode === 'player' && LOWER_IS_BETTER.has(row.index) ? ' ↓' : ''}
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

		{#if mode === 'mixed'}
			<p class="mt-3 text-xs leading-relaxed text-[var(--color-faint)]">
				This board holds both players and staff, and the two carry different sheets — 54 player
				attributes against the 52-item non-player one, which do not line up. Only the figures
				both actually have are shown. Pin one kind at a time to compare attribute by attribute.
			</p>
		{:else if mode === 'staff' && !tendenciesDecoded}
			<p class="mt-3 text-xs leading-relaxed text-[var(--color-faint)]">
				The tendency half of these sheets has moved off the editor's 1&ndash;20 scale — an aged
				career rewrites it onto an internal scale that is not yet decoded — so it is left out
				rather than compared. The coaching half above still reads on the editor's own scale.
			</p>
		{:else}
			<p class="mt-3 text-xs leading-relaxed text-[var(--color-faint)]">
				{#if mode === 'player'}
					A ↓ marks the two attributes where less is better, so the highlight points at the right
					player.
				{/if}
				Rows where everyone matches, or where only one person has a decoded value, are left
				unmarked rather than declaring a winner out of nothing.
			</p>
		{/if}
	</div>
</div>
