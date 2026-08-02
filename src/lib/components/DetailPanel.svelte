<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import AttributeGrid from './AttributeGrid.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { shortlists } from '$lib/classes/Shortlists.svelte';

	const player = $derived(scout.selectedPlayer);
	const club = $derived(scout.selectedClub);
	const shortlisted = $derived(player !== null && shortlists.activeMembers.has(player.name));
	/** Shortlists stored in the save file itself, editable in place. */
	const gameLists = $derived(scout.summary?.game_shortlists ?? []);
</script>

{#if player || club}
	<aside class="flex w-72 shrink-0 flex-col border-l border-[var(--color-line)] bg-[var(--color-panel)]">
		<div class="flex items-start justify-between gap-2 px-4 pt-4 pb-3">
			<h2 class="font-display text-lg leading-tight text-[var(--color-bright)]">
				{player?.name ?? club?.name}
			</h2>
			<button
				type="button"
				class="text-lg leading-none text-[var(--color-faint)] hover:text-[var(--color-bright)]"
				aria-label="Close details"
				onclick={() => (scout.selectedId = null)}>×</button
			>
		</div>

		<div class="flex-1 overflow-y-auto px-4 pb-4">
			{#if player}
				<dl class="mb-4 grid grid-cols-2 gap-x-4 gap-y-3">
					<div>
						<dt class="eyebrow">Age</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.age}</dd>
					</div>
					<div>
						<dt class="eyebrow">Born</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.born}</dd>
					</div>
					<div>
						<dt class="eyebrow">Nation</dt>
						<dd class="text-sm text-[var(--color-bright)]" title="Identifier {player.nation_id}">
							{player.nation || `#${player.nation_id}`}
						</dd>
					</div>
					<div>
						<dt class="eyebrow">Club</dt>
						<dd class="text-sm text-[var(--color-bright)]">{player.club || '—'}</dd>
					</div>
					{#if player.wage !== null}
						<div>
							<dt class="eyebrow">Wage</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">£{player.wage.toLocaleString()}/w</dd>
						</div>
					{/if}
					{#if player.contract_until}
						<div>
							<dt class="eyebrow">Contract until</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">{player.contract_until}</dd>
						</div>
					{/if}
					<div>
						<dt class="eyebrow">Current ability</dt>
						<dd class="tabular text-lg text-[var(--color-bright)]">{player.ability ?? '\u2014'}</dd>
					</div>
					<div>
						<dt class="eyebrow">Max ability</dt>
						<dd class="tabular text-lg text-[var(--color-signal)]">{player.potential ?? '\u2014'}</dd>
					</div>
				</dl>

				<div class="mb-4"><AbilityBar ability={player.ability} potential={player.potential} /></div>

				{#if player.positions.length > 0}
					<div class="mb-4">
						<h4 class="eyebrow mb-1.5">Positions</h4>
						<div class="flex flex-wrap gap-1">
							{#each player.positions as pos (pos)}
								<span
									class="rounded-[2px] border border-[var(--color-hivis-dim)] px-1.5 py-0.5 text-xs
										text-[var(--color-hivis)]">{pos}</span
								>
							{/each}
						</div>
					</div>
				{/if}

				{#if player.professionalism !== null}
					<div class="mb-4">
						<h4 class="eyebrow mb-1.5">Hidden personality</h4>
						<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Professionalism</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.professionalism}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Loyalty</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.loyalty}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Adaptability</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.adaptability}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Controversy</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.controversy}</dd>
							</div>
						</dl>
					</div>
				{/if}

				<AttributeGrid attributes={player.attributes} />

				<button
					type="button"
					class="mt-5 w-full rounded-[2px] border py-1.5 text-xs transition-colors
						{shortlisted
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}
						disabled:cursor-not-allowed disabled:opacity-40"
					disabled={!shortlists.active}
					onclick={() => shortlists.toggle(player.name)}
				>
					{#if !shortlists.active}
						Create a shortlist first
					{:else if shortlisted}
						Remove from {shortlists.active.name}
					{:else}
						Add to {shortlists.active.name}
					{/if}
				</button>

				{#if gameLists.length > 0 && player.eid !== null}
					<div class="mt-4">
						<h4
							class="eyebrow mb-1.5"
							title="Shortlists inside the save file. Edits write to the save — the untouched original is kept as a .gilet.bak sibling."
						>
							In-save shortlists
						</h4>
						{#if !scout.canEditGameShortlists}
							<p class="text-xs leading-relaxed text-[var(--color-faint)]">
								Read-only: this save's own date could not be read, so a
								date-added field cannot be written honestly.
							</p>
						{:else}
							<div class="space-y-1">
								{#each gameLists as list (list)}
									{@const on = list.players.includes(player.name)}
									<button
										type="button"
										class="flex w-full items-center justify-between rounded-[2px] border py-1 pr-2 pl-2 text-xs transition-colors
											{on
											? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
											: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
										onclick={() => scout.toggleGameShortlist(list, player)}
									>
										<span class="truncate">{list.name ?? '(unnamed)'}</span>
										<span class="tabular ml-2 shrink-0">{on ? '− remove' : '+ add'}</span>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

			{:else if club}
				<dl class="space-y-3">
					<div>
						<dt class="eyebrow">Short name</dt>
						<dd class="text-sm text-[var(--color-bright)]">{club.short_name}</dd>
					</div>
					<div>
						<dt class="eyebrow">Club ID</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{club.club_id}</dd>
					</div>
					<div>
						<dt class="eyebrow">Nation ID</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{club.nation_id}</dd>
					</div>
					{#if club.squad_size > 0}
						<div>
							<dt class="eyebrow">Squad</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">{club.squad_size} players</dd>
						</div>
						<div>
							<dt class="eyebrow">Squad average</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">
								CA {club.average_ability ?? '—'} · PA
								<span class="text-[var(--color-signal)]">{club.average_potential ?? '—'}</span>
							</dd>
						</div>
					{/if}
				</dl>
				<button
					type="button"
					class="mt-5 w-full rounded-[2px] border border-[var(--color-line)] py-1.5 text-xs text-[var(--color-mist)]
						transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]"
					onclick={() => scout.showSquad(club.short_name)}
				>
					Show squad
				</button>
			{/if}
		</div>
	</aside>
{/if}
