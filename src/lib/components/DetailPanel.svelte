<script lang="ts">
	import AbilityBar from './AbilityBar.svelte';
	import AttributeGrid from './AttributeGrid.svelte';
	import BackroomAudit from './BackroomAudit.svelte';
	import StaffGrid from './StaffGrid.svelte';
	import PositionStrip from './PositionStrip.svelte';
	import SquadAudit from './SquadAudit.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { flagsFor, hasFlagData, headroom } from '$lib/utils/flags';
	import { estimateInterest } from '$lib/utils/interest';
	import { formatBill } from '$lib/utils/money';

	const player = $derived(scout.selectedPlayer);
	const room = $derived(player ? headroom(player) : null);
	const flags = $derived(player ? flagsFor(player) : []);
	/** Whether the save said anything at all about this person's traits — the
	 * difference between a clean report and no report. */
	const judged = $derived(player !== null && hasFlagData(player));
	const club = $derived(scout.selectedClub);
	/** Shortlists stored in the save file itself, editable in place. */
	const gameLists = $derived(scout.summary?.game_shortlists ?? []);

	/** A year past the save's own date — what "expiring" means for the interest
	 * estimate below. Null when the save could not say what day it is. */
	const expiryCutoff = $derived.by(() => {
		const base = scout.summary?.game_date;
		if (!base) return null;
		return `${Number(base.slice(0, 4)) + 1}${base.slice(4)}`;
	});
	const myClub = $derived(scout.activeMyClub);
	const myClubReputation = $derived(
		myClub ? (scout.clubs.find((c) => c.eid === myClub.eid)?.reputation ?? null) : null
	);
	const expiring = $derived(
		player !== null &&
			expiryCutoff !== null &&
			player.contract_until !== '' &&
			player.contract_until <= expiryCutoff
	);
	/** Gilet's estimate of this player's openness to joining "my club" — only
	 * offered once one is set, since it has nothing to compare against otherwise. */
	const interest = $derived(
		player && myClub ? estimateInterest(player, myClubReputation, expiring) : null
	);
</script>

{#if player || club}
	<aside class="flex w-72 shrink-0 flex-col border-l border-[var(--color-line)] bg-[var(--color-panel)]">
		<div class="flex items-start justify-between gap-2 px-4 pt-4 pb-3">
			<h2 class="font-display text-lg leading-tight text-[var(--color-bright)]">
				{#if player?.stub}
					<span class="text-[var(--color-faint)] italic">Unnamed — non-contract</span>
				{:else}
					{player?.name ?? club?.name}
				{/if}
			</h2>
			<button
				type="button"
				class="text-lg leading-none text-[var(--color-faint)] hover:text-[var(--color-bright)]"
				aria-label="Close details"
				onclick={() => scout.select(null)}>×</button
			>
		</div>

		<div class="flex-1 overflow-y-auto px-4 pb-4">
			{#if player}
				<dl class="mb-4 grid grid-cols-2 gap-x-4 gap-y-3">
					<div>
						<dt class="eyebrow">Age</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.age ?? '—'}</dd>
					</div>
					<div>
						<dt class="eyebrow">Born</dt>
						<dd class="tabular text-sm text-[var(--color-bright)]">{player.born || '—'}</dd>
					</div>
					<div>
						<dt class="eyebrow">Nation</dt>
						<dd class="text-sm text-[var(--color-bright)]" title="Identifier {player.nation_id}">
							{player.nation || `#${player.nation_id}`}
						</dd>
					</div>
					<div>
						<dt class="eyebrow">Club</dt>
						<dd class="text-sm text-[var(--color-bright)]">
							{player.club || '—'}
							{#if player.squad_level && player.squad_level !== 'First Team'}
								<span
									class="text-[var(--color-faint)]"
									title="Bound through the club's {player.squad_level} list, not the first team"
								>
									· {player.squad_level}
								</span>
							{/if}
						</dd>
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
					{#if room !== null}
						<div>
							<dt class="eyebrow">Room to grow</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">+{room}</dd>
						</div>
					{/if}
				</dl>

				<div class="mb-4"><AbilityBar ability={player.ability} potential={player.potential} /></div>

				{#if player.position_ratings.length > 0}
					<div class="mb-4">
						<h4 class="eyebrow mb-1.5">Positions</h4>
						<PositionStrip ratings={player.position_ratings} />
					</div>
				{:else if player.positions.length > 0}
					<!-- No slot ratings decoded, but the naturals list survived. -->
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

				{#if myClub && !player.stub && player.is_player}
					<div class="mb-4">
						<h4
							class="eyebrow mb-1.5"
							title="Gilet's estimate, not FM's own figure — built from the reputation gap with {myClub.name}, plus ambition, loyalty, adaptability and contract status. Never dressed up as the save's own number."
						>
							Interest in {myClub.shortName}
						</h4>
						{#if interest === null}
							<p class="text-xs leading-relaxed text-[var(--color-faint)]">
								{myClubReputation === null
									? `${myClub.name}'s reputation is undecoded, so there is nothing to weigh this against.`
									: "This player's reputation is undecoded, so there is nothing to estimate from."}
							</p>
						{:else}
							<div class="flex items-center gap-2">
								<span
									class="rounded-[2px] border px-1.5 py-0.5 text-xs
										{interest.score >= 20
										? 'border-[var(--color-signal-dim)] text-[var(--color-signal)]'
										: interest.score <= -20
											? 'border-[var(--color-hivis-dim)] text-[var(--color-hivis)]'
											: 'border-[var(--color-line)] text-[var(--color-mist)]'}"
								>
									{interest.band}
								</span>
								<span class="tabular text-xs text-[var(--color-faint)]">{interest.score}</span>
							</div>
							<p class="mt-1 text-xs leading-relaxed text-[var(--color-faint)]">
								{interest.reasons.join(' · ')}
							</p>
						{/if}
					</div>
				{/if}

				{#if judged}
					<div class="mb-4">
						<h4
							class="eyebrow mb-1.5"
							title="Gilet's reading of values the save stores. The numbers are FM's; the line each one has to cross to earn a mention is this app's."
						>
							Scout's flags
						</h4>
						{#if flags.length === 0}
							<p class="text-xs leading-relaxed text-[var(--color-faint)]">
								Nothing stands out either way &mdash; no red flags, and no trait strong
								enough to call a selling point.
							</p>
						{:else}
							<div class="flex flex-wrap gap-1">
								{#each flags as flag (flag.key)}
									<span
										class="rounded-[2px] border px-1.5 py-0.5 text-xs
											{flag.tone === 'risk'
											? 'border-[var(--color-hivis-dim)] text-[var(--color-hivis)]'
											: 'border-[var(--color-signal-dim)] text-[var(--color-signal)]'}"
										title={flag.note}
									>
										{flag.label}
										<span class="tabular ml-0.5 text-[var(--color-mist)]">{flag.value}</span>
									</span>
								{/each}
							</div>
						{/if}
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
								<dt class="text-[var(--color-mist)]">Ambition</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.ambition}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Loyalty</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.loyalty}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Pressure</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.pressure}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Adaptability</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.adaptability}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Sportsmanship</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.sportsmanship}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Temperament</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.temperament}</dd>
							</div>
							<div class="flex justify-between">
								<dt class="text-[var(--color-mist)]">Controversy</dt>
								<dd class="tabular text-[var(--color-bright)]">{player.controversy}</dd>
							</div>
						</dl>
					</div>
				{/if}

				<div class="mb-4">
					<button
						type="button"
						class="w-full rounded-[2px] border py-1.5 text-xs transition-colors
							{scout.isPinned(player.id)
							? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
							: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]'}
							disabled:cursor-not-allowed disabled:opacity-40"
						disabled={!scout.isPinned(player.id) && scout.compareFull}
						title={scout.compareFull && !scout.isPinned(player.id)
							? `The board holds ${scout.compareLimit}. Remove one first.`
							: 'Pin this player to the compare board'}
						onclick={() => scout.togglePinned(player)}
					>
						{scout.isPinned(player.id) ? '− Remove from compare' : '+ Compare'}
					</button>
				</div>

				<!-- Actions sit above the attribute grid: below it they were past
					the fold on every save, and nobody found them. FM's in-save
					shortlists hold players, so a staff person gets no add button. -->
				{#if gameLists.length > 0 && player.eid !== null && (player.is_player || player.staff === null)}
					<div class="mb-4">
						<h4
							class="eyebrow mb-1.5"
							title="Edits write to the save file — FM sees them on next load. The untouched original is kept as a .gilet.bak sibling."
						>
							Shortlists
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
							<!-- A failed write must be said out loud here, where the click
								happened — the only other error surface is the load screen. -->
							{#if scout.error}
								<p class="mt-1.5 text-xs leading-relaxed text-[var(--color-hivis)]">{scout.error}</p>
							{/if}
						{/if}
					</div>
				{/if}

				{#if player.stub}
					<p class="text-xs leading-relaxed text-[var(--color-faint)]">
						The save stores this squad member as a stub — a non-contract signing
						with no full record. Their name, age and attributes exist in the game
						but are not yet decoded, so nothing is shown rather than guessed.
					</p>
				{:else if player.staff}
					<StaffGrid sheet={player.staff} />
				{:else}
					<AttributeGrid attributes={player.attributes} />
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
					{#if club.reputation !== null}
						<div>
							<dt class="eyebrow" title="The club's reputation on FM's 0-10000 scale, from the save's own roster table">
								Reputation
							</dt>
							<dd class="tabular text-sm text-[var(--color-signal)]">{club.reputation.toLocaleString()}</dd>
						</div>
					{/if}
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
						<div>
							<dt class="eyebrow" title="Mean age of the squad players whose birth date decoded">
								Average age
							</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">{club.average_age ?? '—'}</dd>
						</div>
						<div>
							<dt
								class="eyebrow"
								title={club.wages_known < club.squad_size
									? `${club.wages_known} of ${club.squad_size} squad wages decoded — a floor, not the club's outgoings. Staff wages are not decoded at all.`
									: 'Every squad wage decoded. Staff wages are not decoded at all, so this is the playing bill.'}
							>
								Wage bill
							</dt>
							<dd class="tabular text-sm text-[var(--color-bright)]">
								{formatBill(club.wage_bill, '—')}{club.wage_bill === null ? '' : '/w'}
							</dd>
						</div>
					{/if}
				</dl>
				<button
					type="button"
					class="mt-5 w-full rounded-[2px] border border-[var(--color-line)] py-1.5 text-xs text-[var(--color-mist)]
						transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]"
					onclick={() => scout.showSquad({ eid: club.eid, shortName: club.short_name })}
				>
					Show squad
				</button>
				{#if club.eid !== null}
					{@const isMine = scout.activeMyClub?.eid === club.eid}
					<button
						type="button"
						class="mt-2 w-full rounded-[2px] border py-1.5 text-xs transition-colors
							{isMine
							? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
							: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]'}"
						title={isMine
							? 'This is set as your club. Click to clear the choice.'
							: 'Scout as if you manage this club — unlocks the squad and weakness tools in the sidebar'}
						onclick={() => scout.setMyClub(isMine ? null : club.eid)}
					>
						{isMine ? 'Your club ✓' : 'Set as my club'}
					</button>
				{/if}
				<SquadAudit {club} />
				<BackroomAudit {club} />
			{/if}
		</div>
	</aside>
{/if}
