<script lang="ts">
	/**
	 * The traits half of the second filter row: how many positions they cover,
	 * how readily they learn a new one, the two hidden personality drivers, a
	 * dead-ball skill, and the active scoring profile's own minimum.
	 *
	 * Cover counts at the tier chosen on the row above, so one control governs
	 * both "a DM" and "can do a job at DM".
	 */
	import { scout } from '$lib/classes/Scout.svelte';
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { SET_PIECES } from '$lib/utils/attributes';
</script>

<div class="flex items-center gap-1">
	<span
		class="eyebrow mr-1"
		title="How many positions the player is rated in, at the tier chosen on the row above — 15+ for their own positions, 10+ for what they can cover"
	>
		Covers
	</span>
	<input
		type="number"
		min="1"
		max="15"
		placeholder="min"
		bind:value={scout.filters.minPositions}
		aria-label="Minimum positions covered"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>
	<span
		class="eyebrow mr-1 ml-2"
		title="The Versatility attribute — how readily the player learns a new role, which is not the same as how many they already play"
	>
		Versatility
	</span>
	<input
		type="number"
		min="1"
		max="20"
		placeholder="min"
		bind:value={scout.filters.minVersatility}
		aria-label="Minimum versatility"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>
</div>

<div class="flex items-center gap-1">
	<span
		class="eyebrow mr-1"
		title="Hidden Professionalism, 1-20 — the strongest single brake on, or driver of, a young player reaching their potential. Anyone whose personality run didn't decode fails the bound rather than passing at zero."
	>
		Prof
	</span>
	<input
		type="number"
		min="1"
		max="20"
		placeholder="min"
		bind:value={scout.filters.minProfessionalism}
		aria-label="Minimum professionalism"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>
	<span
		class="eyebrow mr-1 ml-2"
		title="Hidden Ambition, 1-20 — whether they want the step up, which makes them both gettable and drivable"
	>
		Ambition
	</span>
	<input
		type="number"
		min="1"
		max="20"
		placeholder="min"
		bind:value={scout.filters.minAmbition}
		aria-label="Minimum ambition"
		class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
			placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
	/>
</div>

<div class="flex items-center gap-1">
	<select
		value={scout.filters.setPiece}
		aria-label="Filter by a set-piece skill"
		title="Filter on one dead-ball skill — corners, free kicks, penalties, long throws. Picking one arms the minimum at 14, a specialist's level; the box stays editable"
		class="rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
			text-[var(--color-mist)] focus:border-[var(--color-hivis)] focus:outline-none"
		onchange={(event) => {
			const key = event.currentTarget.value;
			scout.filters.setPiece = key === '' ? null : key;
			// A skill with no minimum filters nothing, so picking one arms it at
			// a specialist's level; the box stays editable.
			if (key !== '' && scout.filters.minSetPiece === null) scout.filters.minSetPiece = 14;
			if (key === '') scout.filters.minSetPiece = null;
		}}
	>
		<option value="">Any set piece</option>
		{#each SET_PIECES as skill (skill.key)}
			<option value={skill.key}>{skill.label}</option>
		{/each}
	</select>
	{#if scout.filters.setPiece !== null}
		<input
			type="number"
			min="1"
			max="20"
			placeholder="min"
			bind:value={scout.filters.minSetPiece}
			aria-label="Minimum set-piece rating"
			class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
				placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
		/>
	{/if}
</div>

{#if profiles.active}
	<div class="flex items-center gap-1">
		<span
			class="eyebrow mr-1"
			title="Minimum score under your active profile — a weighted average of the attributes you chose, on the same 1-20 scale. Your weights, not an FM figure; anyone missing the sheet it weights is excluded"
		>
			{profiles.active.name}
		</span>
		<input
			type="number"
			min="1"
			max="20"
			step="0.5"
			placeholder="min"
			bind:value={scout.filters.minScore}
			aria-label="Minimum score"
			class="tabular w-14 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
				placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
		/>
	</div>
{/if}
