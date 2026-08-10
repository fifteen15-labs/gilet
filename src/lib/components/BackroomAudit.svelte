<script lang="ts">
	/**
	 * A club's backroom the way its staff screen reads: who is in each
	 * department, how strong they are, and where there is nobody at all.
	 *
	 * Only the save's own three department lists and the roster's manager seat.
	 * Staff bound to the club with no department are reported apart rather than
	 * spread across the three, and a department nobody in has a decoded sheet
	 * for shows no average rather than an average of nothing.
	 */
	import { scout } from '$lib/classes/Scout.svelte';
	import { auditBackroom } from '$lib/utils/audit';
	import { clubPeople, type Club, type Player } from '$lib/tauri/commands';
	import type { Filters } from '$lib/utils/filter';

	type Props = { club: Club };
	const { club }: Props = $props();

	/** The people at this club, fetched from the backend where the rows live.
	 * The name guard drops a stale reply when the panel moves to another club
	 * before the first fetch lands. */
	let people = $state.raw<Player[]>([]);
	$effect(() => {
		const wanted = club.short_name;
		void clubPeople(wanted, club.eid).then((rows) => {
			if (wanted === club.short_name) people = rows;
		});
	});

	const audit = $derived(auditBackroom(club, people));

	/** Jumps to the people table showing this club's staff, by department. */
	function drill(role: Filters['staffRole']) {
		scout.screen(
			club.eid !== null
				? { clubEid: club.eid, kind: 'staff', staffRole: role }
				: { query: club.short_name, kind: 'staff', staffRole: role },
			'ability'
		);
	}
</script>

<div class="mt-5">
	<h4 class="eyebrow mb-2" title="From the save's own department lists in the club record, plus the roster table's manager seat">
		Backroom
	</h4>

	{#if audit.counted === 0}
		<p class="text-xs leading-relaxed text-[var(--color-faint)]">
			No staff bound to this club. Either it employs none in the loaded game world, or its
			department lists did not decode — which is not the same thing, so nothing is counted
			either way.
		</p>
	{:else}
		<div class="mb-2">
			<button
				type="button"
				class="flex w-full items-center justify-between rounded-[2px] border border-[var(--color-line)] px-2 py-1
					text-xs text-[var(--color-mist)] transition-colors hover:border-[var(--color-hivis)]
					disabled:cursor-not-allowed disabled:opacity-40"
				disabled={audit.manager === null}
				title="The manager from the roster table's own seat, which is outside the three department lists"
				onclick={() => drill('Manager')}
			>
				<span class="eyebrow">Manager</span>
				<span class="truncate pl-2">{audit.manager?.name ?? 'vacant'}</span>
			</button>
		</div>

		<div class="space-y-1">
			{#each audit.departments as dept (dept.role)}
				<button
					type="button"
					class="flex w-full items-center gap-2 rounded-[2px] border px-2 py-1 text-xs transition-colors
						{dept.count === 0
						? 'border-[var(--color-hivis-dim)] text-[var(--color-hivis)]'
						: 'border-[var(--color-line)] text-[var(--color-mist)] hover:border-[var(--color-hivis)]'}
						disabled:cursor-not-allowed"
					disabled={dept.count === 0}
					title={dept.count === 0
						? `Nobody in ${club.short_name}'s ${dept.role.toLowerCase()} department`
						: `Mean non-player CA ${dept.averageAbility ?? '—'} · best world reputation ${dept.topReputation ?? '—'}`}
					onclick={() => drill(dept.role)}
				>
					<span class="flex-1 text-left">{dept.role}</span>
					<span class="tabular text-[var(--color-faint)]">
						CA {dept.averageAbility ?? '—'}
					</span>
					<span class="tabular w-4 text-right">{dept.count}</span>
				</button>
			{/each}
		</div>

		<p class="mt-2 text-xs leading-relaxed text-[var(--color-faint)]">
			{#if audit.gaps.length > 0}
				Nobody at all in {audit.gaps.join(', ').toLowerCase()}.
			{/if}
			{#if audit.thin.length > 0}
				One person covers {audit.thin.join(', ').toLowerCase()} — Gilet's line, not FM's: a
				department of one stops working the week they do.
			{/if}
			{#if audit.unassigned > 0}
				{audit.unassigned}
				{audit.unassigned === 1 ? 'other member of staff is' : 'other staff are'} employed here
				with no department decoded, so they are in none of the counts above.
			{/if}
			{#if audit.gaps.length === 0 && audit.thin.length === 0 && audit.unassigned === 0}
				Every department has someone in it.
			{/if}
		</p>
	{/if}
</div>
