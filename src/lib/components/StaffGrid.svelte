<script lang="ts">
	import { staffAttributeNames, type StaffSheet } from '$lib/tauri/commands';

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
	const COACHING_FROM = 26;
	const tendencies = $derived(entries.filter((e) => e.index < COACHING_FROM));
	const coaching = $derived(entries.filter((e) => e.index >= COACHING_FROM));

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
	{@render group('Tendencies', tendencies)}

	<p class="text-xs leading-relaxed text-[var(--color-faint)]">
		The non-player sheet, on FM's 1-20 scale, in the pre-game editor's own order. Where a person's
		database row is blank the game generates these at career start, so a value is what the save
		holds, not what the editor shows.
	</p>
</div>
