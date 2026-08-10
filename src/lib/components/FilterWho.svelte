<script lang="ts">
	/**
	 * The first row of the filter bar: who you are looking for. Kind, role,
	 * gender, position, nation, contract status, red flags and which in-save
	 * shortlist — everything that narrows the population before any judgement
	 * of how good they are, which is the second row's job.
	 */
	import { scout } from '$lib/classes/Scout.svelte';
	import { type Filters } from '$lib/utils/filter';
	import { POSITIONS } from '$lib/utils/positions';

	type Props = {
		/** A year past the save's own date — what "expiring" means here. */
		expiryCutoff: string;
	};
	const { expiryCutoff }: Props = $props();

	/** The flag rules read hidden attributes and the personality run; without
	 * either there is nothing to flag, so the toggles hide rather than filter
	 * everyone out. Derived by the backend at load, since the rows live there. */
	const flagsKnown = $derived(scout.summary?.flags_known ?? false);
	/** Gender derives from the save's own squads; without women's football it
	 * stays unknown and the filter hides rather than lying. */
	const genderKnown = $derived(scout.summary?.gender_known ?? false);
	const nations = $derived(scout.summary?.nations ?? []);
	const gameLists = $derived(scout.summary?.game_shortlists ?? []);
	/** Clubs with a validated entity id, alphabetical — what a `clubEid`
	 * filter can actually key on. The handful without one have no id to
	 * filter by and would only clutter the list. */
	const clubs = $derived(
		[...scout.clubs]
			.filter((c) => c.eid !== null)
			.sort((a, b) => a.short_name.localeCompare(b.short_name))
	);

	function setStaffRole(value: string) {
		scout.filters.staffRole = value as Filters['staffRole'];
	}

	function setSquadLevel(value: string) {
		scout.filters.squadLevel = value as Filters['squadLevel'];
	}
</script>

<!-- A hairline between clusters: each marks a genuine change of subject
	(who they are → where they play → who's watching them → their status),
	not decoration. -->
{#snippet divider()}
	<div class="h-5 w-px shrink-0 bg-[var(--color-line)]" aria-hidden="true"></div>
{/snippet}

<div class="flex flex-wrap items-center gap-2">
	<input
		type="search"
		bind:value={scout.filters.query}
		placeholder={scout.tab === 'clubs' ? 'Search clubs' : 'Search players'}
		aria-label="Search by name"
		title="Search by name or club — accents don't matter, so 'mbappe' finds Mbappé and 'man city' lists City's squad"
		class="w-56 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2.5 py-1.5 text-sm
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>

	{#if scout.tab === 'people'}
		{@render divider()}
		<div class="flex items-center gap-1">
			{#each [
				{ k: 'all', label: 'All', tip: 'Everyone in the save — players, staff, and undecoded squad fillers' },
				{ k: 'players', label: 'Players', tip: 'Only people with a player attribute block' },
				{ k: 'staff', label: 'Staff', tip: 'Only non-players — coaches, physios, scouts, managers. Unlocks the role and reputation filters, and the CA/PA bounds read their non-player ability' }
			] as opt (opt.k)}
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.kind === opt.k
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.kind === opt.k}
					title={opt.tip}
					onclick={() => (scout.filters.kind = opt.k === 'players' ? 'players' : opt.k === 'staff' ? 'staff' : 'all')}
				>
					{opt.label}
				</button>
			{/each}
		</div>

		{#if scout.filters.kind === 'staff'}
			<select
				value={scout.filters.staffRole ?? 'any'}
				aria-label="Backroom role"
				title="The save's own backroom groups: the manager seat, or the coaching, medical and recruitment department lists. Staff the save gives no department for only show under Any role."
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => setStaffRole(event.currentTarget.value)}
			>
				<option value="any">Any role</option>
				<option value="Manager">Manager</option>
				<option value="Coaching">Coaching</option>
				<option value="Medical">Medical</option>
				<option value="Recruitment">Recruitment</option>
			</select>
		{/if}

		{#if genderKnown}
			{@render divider()}
			<div
				class="flex items-center gap-1"
				title="Gender derives from the save's own squads. Anyone the save can't settle only shows under Everyone — showing a woman under Men would be a guess"
			>
				{#each [{ k: 'all', label: 'Everyone' }, { k: 'men', label: 'Men' }, { k: 'women', label: 'Women' }] as opt (opt.k)}
					<button
						type="button"
						class="rounded-[2px] border px-2 py-1 text-xs transition-colors
							{scout.filters.gender === opt.k
							? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
							: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
						aria-pressed={scout.filters.gender === opt.k}
						onclick={() =>
							(scout.filters.gender = opt.k === 'men' ? 'men' : opt.k === 'women' ? 'women' : 'all')}
					>
						{opt.label}
					</button>
				{/each}
			</div>
		{/if}

		{@render divider()}
		<div class="flex items-center gap-1">
			<select
				bind:value={scout.filters.position}
				aria-label="Filter by position"
				title="Only players who play this position. The tier beside it decides what counts: their own position, or one they can be asked to fill. Staff have no positions, so any position filter hides them"
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
			>
				<option value={null}>Any position</option>
				{#each POSITIONS as p (p)}
					<option value={p}>{p}</option>
				{/each}
			</select>
			<select
				value={scout.filters.positionTier ?? 'natural'}
				aria-label="Position rating tier"
				title="FM's own two readings of a position rating. Natural (15+) is a player's own position; can cover (10+) is one they can be asked to fill. Governs the Covers count on the row below too."
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) =>
					(scout.filters.positionTier =
						event.currentTarget.value === 'accomplished' ? 'accomplished' : 'natural')}
			>
				<option value="natural">natural 15+</option>
				<option value="accomplished">can cover 10+</option>
			</select>
		</div>

		{@render divider()}
		<select
			bind:value={scout.filters.nationId}
			aria-label="Filter by nationality"
			title="By nationality, using the save's own numbering. Nations the parser hasn't named yet appear as raw identifiers at the bottom — they still filter correctly"
			class="max-w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
				text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
		>
			<option value={null}>Any nation</option>
			{#each nations as n (n.id)}
				<option value={n.id}>{n.name}</option>
			{/each}
		</select>

		{@render divider()}
		<div class="flex items-center gap-1">
			<select
				bind:value={scout.filters.clubEid}
				aria-label="Filter by club"
				title="Only people at this club, by the save's own club identifier rather than the name shown — two clubs can share a short name. B and youth squads bind to the same club as the first team, so this is the whole academy, not just who plays on Saturday"
				class="max-w-40 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
			>
				<option value={null}>Any club</option>
				{#each clubs as c (c.eid)}
					<option value={c.eid}>{c.short_name}</option>
				{/each}
			</select>
			<select
				value={scout.filters.squadLevel}
				aria-label="Squad level"
				title="Which of a club's own squad lists placed this person there: the first team, a B/reserve side, the youth squad, or — for a club outside the loaded leagues — its own senior list. A person the squad table couldn't place shows under Any level only"
				class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => setSquadLevel(event.currentTarget.value)}
			>
				<option value="any">Any level</option>
				<option value="First Team">First team</option>
				<option value="B Team">B / reserve</option>
				<option value="U21">U21 / development</option>
				<option value="Youth">Youth</option>
				<option value="Out of League">Unloaded league</option>
			</select>
		</div>

		{#if gameLists.length > 0}
			{@render divider()}
			<select
				value={scout.filters.shortlist === null || scout.filters.shortlist === undefined
					? 'any'
					: `list:${scout.filters.shortlist}`}
				aria-label="Filter to one in-save shortlist"
				title="Show only the members of one of FM's own shortlists in this save, so a list built in the game can be sorted, scored and compared here. Members whose person record didn't decode are not in it to show."
				class="max-w-36 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
					text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
				onchange={(event) => {
					const value = event.currentTarget.value;
					// The unnamed default list is the empty string, so "no
					// filter" needs a key of its own rather than sharing it.
					scout.filters.shortlist = value === 'any' ? null : value.slice('list:'.length);
				}}
			>
				<option value="any">Any shortlist</option>
				{#each gameLists as list (list)}
					<option value="list:{list.name ?? ''}">
						{list.name ?? '(unnamed)'} ({list.players.length})
					</option>
				{/each}
			</select>
		{/if}

		{@render divider()}
		<div class="flex items-center gap-1">
			<button
				type="button"
				class="rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.contract === 'free'
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.contract === 'free'}
				title="No contract, no wage, no club. For staff — whose wages aren't decoded — this reads as 'no club found', so a few obscure employed staff can slip in"
				onclick={() => (scout.filters.contract = scout.filters.contract === 'free' ? 'any' : 'free')}
			>
				Free agents
			</button>
			<button
				type="button"
				class="rounded-[2px] border px-2 py-1 text-xs transition-colors
					{scout.filters.contract === 'expiring'
					? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
					: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
				aria-pressed={scout.filters.contract === 'expiring'}
				title="Contract ends within a year of the save's own date. Contracts the parser couldn't read never count as expiring — an unknown deal is not a bargain"
				onclick={() => {
					if (scout.filters.contract === 'expiring') {
						scout.filters.contract = 'any';
						scout.filters.expiryCutoff = null;
					} else {
						scout.filters.contract = 'expiring';
						scout.filters.expiryCutoff = expiryCutoff;
					}
				}}
			>
				Expiring
			</button>
		</div>

		{#if flagsKnown}
			{@render divider()}
			<div class="flex items-center gap-1">
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.risk === 'clean'
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.risk === 'clean'}
					title="Only players the save can vouch for — no injury proneness, no temperament or professionalism problems. Staff and undecoded stubs drop out: no reading is not a good one."
					onclick={() => (scout.filters.risk = scout.filters.risk === 'clean' ? 'any' : 'clean')}
				>
					No red flags
				</button>
				<button
					type="button"
					class="rounded-[2px] border px-2 py-1 text-xs transition-colors
						{scout.filters.risk === 'flagged'
						? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-faint)]'}"
					aria-pressed={scout.filters.risk === 'flagged'}
					title="Only players carrying at least one red flag — for auditing a squad you already own"
					onclick={() => (scout.filters.risk = scout.filters.risk === 'flagged' ? 'any' : 'flagged')}
				>
					Red flags
				</button>
			</div>
		{/if}
	{/if}
</div>
