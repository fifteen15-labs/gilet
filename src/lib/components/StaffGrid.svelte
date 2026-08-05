<script lang="ts">
	import { staffAttributeNames, type StaffSheet } from '$lib/tauri/commands';
	import { STAFF_COACHING_FROM } from '$lib/utils/attributes';

	type Props = { sheet: StaffSheet };
	const { sheet }: Props = $props();

	// The same 52 names for every person in the save, so they are fetched once
	// and shared rather than sent down with each row.
	let names = $state<string[]>([]);
	$effect(() => {
		staffAttributeNames().then((n) => (names = n));
	});

	type Entry = { index: number; value: number };
	const entries = $derived(sheet.attributes.map((value, index) => ({ index, value })));
	// The editor's list turns from tendencies to coaching between Width and
	// Coaching, which is also where the save changes storage scale.
	const tendencies = $derived(entries.filter((e) => e.index < STAFF_COACHING_FROM));
	const coaching = $derived(entries.filter((e) => e.index >= STAFF_COACHING_FROM));
	/** The tendency half is stored raw 1-20 on a day-one save, but an aged
	 * save moves it onto an internal scale nobody has decoded — Klopp reads
	 * 98 where the editor caps at 20. A value past 20 proves the whole half
	 * is off the editor scale for this person, and a number on an unknown
	 * scale is no reading. */
	const tendenciesDecoded = $derived(tendencies.every((e) => e.value <= 20));

	const standing = $derived([
		{ label: 'Ability', value: sheet.currentAbility },
		{ label: 'Potential', value: sheet.potentialAbility },
		{ label: 'Reputation (current)', value: sheet.currentReputation },
		{ label: 'Reputation (home)', value: sheet.homeReputation },
		{ label: 'Reputation (world)', value: sheet.worldReputation }
	]);

	/** FM's own colouring: weak values recede, strong ones stand out. */
	function tone(value: number): string {
		if (value >= 15) return 'text-[var(--color-signal)]';
		if (value >= 10) return 'text-[var(--color-bright)]';
		if (value >= 6) return 'text-[var(--color-mist)]';
		return 'text-[var(--color-faint)]';
	}
</script>

{#snippet group(title: string, rows: Entry[])}
	<div class="mb-4">
		<h4 class="eyebrow mb-1.5">{title}</h4>
		<div class="grid grid-cols-2 gap-x-4 gap-y-0.5">
			{#each rows as entry (entry.index)}
				<div
					class="flex items-baseline justify-between border-b border-[var(--color-line-soft)] py-0.5"
				>
					{#if names[entry.index]}
						<span class="text-xs text-[var(--color-mist)]">{names[entry.index]}</span>
					{:else}
						<!-- The pre-game editor leaves these two rows unnamed itself. -->
						<span class="text-xs text-[var(--color-faint)] italic">slot {entry.index}</span>
					{/if}
					<span class="tabular text-xs {tone(entry.value)}">{entry.value}</span>
				</div>
			{/each}
		</div>
	</div>
{/snippet}

<div>
	<h4 class="eyebrow mb-1.5">Standing</h4>
	<div class="mb-4 grid grid-cols-2 gap-x-4 gap-y-0.5">
		{#each standing as row (row.label)}
			<div
				class="flex items-baseline justify-between border-b border-[var(--color-line-soft)] py-0.5"
			>
				<span class="text-xs text-[var(--color-mist)]">{row.label}</span>
				<span class="tabular text-xs text-[var(--color-bright)]">{row.value}</span>
			</div>
		{/each}
	</div>

	{@render group('Coaching and knowledge', coaching)}
	{#if tendenciesDecoded}
		{@render group('Tendencies', tendencies)}
	{:else}
		<div class="mb-4">
			<h4 class="eyebrow mb-1.5">Tendencies</h4>
			<p class="text-xs leading-relaxed text-[var(--color-faint)]">
				This save's tendency values have moved off the editor's 1&ndash;20 scale — an aged
				career rewrites them onto an internal scale that is not yet decoded — so no number
				here would be honest. The coaching half above still reads on the editor's own scale.
			</p>
		</div>
	{/if}

	<p class="text-xs leading-relaxed text-[var(--color-faint)]">
		The non-player sheet, on FM's 1-20 scale, in the pre-game editor's own order. Where a person's
		database row is blank the game generates these at career start, so a value is what the save
		holds, not what the editor shows.
	</p>
</div>
