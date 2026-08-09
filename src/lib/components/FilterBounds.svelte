<script lang="ts">
	/**
	 * The money-and-ability half of the second filter row: wage, age, Current
	 * and Potential Ability, room to grow, and — under the Staff kind — world
	 * reputation.
	 *
	 * Every bound here excludes an unknown rather than passing it at zero, and
	 * the tooltips say so: a filter that silently dropped undecoded people
	 * would look like a search that found nothing.
	 */
	import { scout } from '$lib/classes/Scout.svelte';

	type Props = {
		/** Whether any ability was decoded; the CA/PA boxes need it. */
		abilityKnown: boolean;
	};
	const { abilityKnown }: Props = $props();

	/** Reputation only exists on a non-player sheet, so the box belongs to the
	 * Staff kind — but a bound already set stays on screen whatever the kind,
	 * because a filter narrowing the table from behind a hidden control is how
	 * a search comes back empty for no visible reason. */
	const showReputation = $derived(
		scout.filters.kind === 'staff' || scout.filters.minReputation !== null
	);
</script>

{#snippet divider()}
	<div class="h-5 w-px shrink-0 bg-[var(--color-line)]" aria-hidden="true"></div>
{/snippet}

<div class="flex items-center gap-1">
	<span
		class="eyebrow mr-1"
		title="Highest weekly wage. Players whose wage the parser could not read are excluded — an unreadable wage is not a cheap one."
	>
		Max wage
	</span>
	<input
		type="number"
		min="0"
		step="500"
		bind:value={scout.filters.maxWage}
		aria-label="Maximum weekly wage"
		class="tabular w-20 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			focus:border-[var(--color-hivis)] focus:outline-none"
	/>
</div>

<div
	class="flex items-center gap-1"
	title="Age on the save's own in-game date. People with no readable birth date — stubs and compacted people — never pass an age cap: an unknown age is not a young one"
>
	<span class="eyebrow mr-1">Max age</span>
	<input
		type="number"
		min="14"
		max="60"
		bind:value={scout.filters.maxAge}
		aria-label="Maximum age"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			focus:border-[var(--color-hivis)] focus:outline-none"
	/>
</div>

{@render divider()}
<div class="flex items-center gap-1" title={abilityKnown ? '' : 'This save has no ability data'}>
	<span
		class="eyebrow mr-1"
		title="Current Ability, 1-200 — a player's own, or the non-player CA for staff. Both are the save's exact figures. Anyone with neither decoded fails the bound rather than passing at zero"
	>
		CA
	</span>
	<input
		type="number"
		min="1"
		max="200"
		placeholder="min"
		disabled={!abilityKnown}
		bind:value={scout.filters.minAbility}
		aria-label="Minimum current ability"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
			disabled:cursor-not-allowed disabled:opacity-40"
	/>
	<span class="text-xs text-[var(--color-faint)]">–</span>
	<input
		type="number"
		min="1"
		max="200"
		placeholder="max"
		disabled={!abilityKnown}
		bind:value={scout.filters.maxAbility}
		aria-label="Maximum current ability"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
			disabled:cursor-not-allowed disabled:opacity-40"
	/>
	<span
		class="eyebrow mr-1 ml-2"
		title="Potential Ability, 1-200 — the ceiling the save assigns, player or staff. A max bound rules out the ones already at the top"
	>
		PA
	</span>
	<input
		type="number"
		min="1"
		max="200"
		placeholder="min"
		disabled={!abilityKnown}
		bind:value={scout.filters.minPotential}
		aria-label="Minimum potential ability"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
			disabled:cursor-not-allowed disabled:opacity-40"
	/>
	<span class="text-xs text-[var(--color-faint)]">–</span>
	<input
		type="number"
		min="1"
		max="200"
		placeholder="max"
		disabled={!abilityKnown}
		bind:value={scout.filters.maxPotential}
		aria-label="Maximum potential ability"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
			disabled:cursor-not-allowed disabled:opacity-40"
	/>
	<span
		class="eyebrow mr-1 ml-2"
		title="Room left to grow: max ability minus current. The development screener's one number."
	>
		Grow
	</span>
	<input
		type="number"
		min="0"
		max="200"
		placeholder="min"
		disabled={!abilityKnown}
		bind:value={scout.filters.minHeadroom}
		aria-label="Minimum headroom, max ability minus current"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none
			disabled:cursor-not-allowed disabled:opacity-40"
	/>
</div>

{#if showReputation}
	{@render divider()}
	<div class="flex items-center gap-1">
		<span
			class="eyebrow mr-1"
			title="Worldwide reputation, 0-200, from the non-player sheet — the reputation that decides who will take your call, as against the one they have at home or at their current club. Players have no decoded reputation, so this bound excludes them."
		>
			World rep
		</span>
		<input
			type="number"
			min="0"
			max="200"
			placeholder="min"
			bind:value={scout.filters.minReputation}
			aria-label="Minimum world reputation"
			class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-void)] px-2 py-1 text-xs
				placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
		/>
	</div>
{/if}
