<script lang="ts">
	import { profiles } from '$lib/classes/Profiles.svelte';
	import { scout } from '$lib/classes/Scout.svelte';
	import { describeProfile } from '$lib/utils/score';

	type Props = { onClose: () => void };
	const { onClose }: Props = $props();

	/** Weights run 1-5. A scout thinks "this matters twice as much", not
	 * "this is 0.37 of the total", and the score is a weighted mean so only
	 * the ratios between them matter. */
	const STEPS = [0, 1, 2, 3, 4, 5];

	const names = $derived(scout.summary?.attribute_names ?? []);
	const goalkeeping = $derived(new Set(scout.summary?.goalkeeping_indices ?? []));

	/** Only named attributes can be weighted: an undecoded index has no meaning
	 * to weight by, and offering "#12" invites a guess. */
	const outfield = $derived(
		names.map((name, index) => ({ name, index })).filter((a) => a.name !== '' && !goalkeeping.has(a.index))
	);
	const keeper = $derived(
		names.map((name, index) => ({ name, index })).filter((a) => a.name !== '' && goalkeeping.has(a.index))
	);

	let draftName = $state(profiles.active?.name ?? '');
	let weights = $state<Record<string, number>>({ ...(profiles.active?.weights ?? {}) });

	const weighted = $derived(Object.values(weights).filter((w) => w > 0).length);
	const preview = $derived(describeProfile({ name: draftName, weights }, names));

	function setWeight(index: number, value: number) {
		if (value === 0) {
			const { [String(index)]: _removed, ...rest } = weights;
			weights = rest;
			return;
		}
		weights = { ...weights, [String(index)]: value };
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		await profiles.save(draftName, weights);
		onClose();
	}
</script>

<div class="flex h-full flex-col">
	<div class="flex items-baseline justify-between border-b border-[var(--color-line)] px-4 py-2.5">
		<h2 class="eyebrow">Scoring profile</h2>
		<button
			type="button"
			class="text-xs text-[var(--color-faint)] hover:text-[var(--color-mist)]"
			onclick={onClose}
		>
			Close
		</button>
	</div>

	<p class="px-4 pt-3 text-xs leading-relaxed text-[var(--color-faint)]">
		Weight the attributes you care about and every player gets a weighted average of them, on the
		same 1&ndash;20 scale. These are <em>your</em> weights: FM's own role ratings use weights Sports
		Interactive has never published, so Gilet ships none rather than guess at them.
	</p>

	<form class="flex min-h-0 flex-1 flex-col" onsubmit={submit}>
		<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
			{#each [{ title: 'Outfield', rows: outfield }, { title: 'Goalkeeping', rows: keeper }] as group (group.title)}
				{#if group.rows.length > 0}
					<h3 class="eyebrow mt-3 mb-1.5">{group.title}</h3>
					{#each group.rows as attr (attr.index)}
						<div class="flex items-center gap-2 py-0.5">
							<span
								class="flex-1 truncate text-xs
									{weights[String(attr.index)] ? 'text-[var(--color-bright)]' : 'text-[var(--color-mist)]'}"
							>
								{attr.name}
							</span>
							<div class="flex items-center gap-0.5">
								{#each STEPS as step (step)}
									<button
										type="button"
										class="tabular h-5 w-5 rounded-[2px] border text-[10px] transition-colors
											{(weights[String(attr.index)] ?? 0) === step
											? 'border-[var(--color-hivis)] text-[var(--color-hivis)]'
											: 'border-[var(--color-line)] text-[var(--color-faint)] hover:border-[var(--color-faint)]'}"
										aria-label="{attr.name} weight {step}"
										aria-pressed={(weights[String(attr.index)] ?? 0) === step}
										onclick={() => setWeight(attr.index, step)}
									>
										{step === 0 ? '–' : step}
									</button>
								{/each}
							</div>
						</div>
					{/each}
				{/if}
			{/each}
		</div>

		<div class="border-t border-[var(--color-line)] px-4 py-3">
			{#if weighted > 0}
				<p class="mb-2 truncate text-xs text-[var(--color-faint)]" title={preview}>
					{weighted} weighted &middot; {preview}
				</p>
			{/if}
			<div class="flex items-center gap-2">
				<input
					bind:value={draftName}
					placeholder="Name this profile"
					aria-label="Profile name"
					class="min-w-0 flex-1 rounded-[2px] border border-[var(--color-line)] bg-[var(--color-panel)] px-2 py-1 text-xs
						placeholder:text-[var(--color-faint)] focus:border-[var(--color-hivis)] focus:outline-none"
				/>
				<button
					type="submit"
					class="rounded-[2px] border border-[var(--color-line)] px-2 py-1 text-xs text-[var(--color-mist)]
						transition-colors hover:border-[var(--color-hivis)] hover:text-[var(--color-hivis)]
						disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-[var(--color-line)]
						disabled:hover:text-[var(--color-mist)]"
					disabled={draftName.trim() === '' || weighted === 0}
				>
					Save
				</button>
			</div>
			{#if profiles.error}
				<p class="mt-2 text-xs text-[var(--color-hivis)]">{profiles.error}</p>
			{/if}
		</div>
	</form>
</div>
