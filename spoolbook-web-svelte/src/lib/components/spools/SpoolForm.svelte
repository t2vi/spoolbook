<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import FilamentPicker from '$lib/components/filament-picker.svelte';
	import { createSpool, getSpool, listAllFilaments, updateSpool } from '$lib/api/client';
	import type { SpoolInput } from '$lib/api/client';
	import type { Filament } from '$lib/api/types';
	import { goto } from '$app/navigation';
	import { numOrNull } from '$lib/utils.js';

	let { id }: { id: number | null } = $props();

	let filaments = $state<Filament[]>([]);
	let selectedFilamentId = $state(0);
	let lotCode = $state('');
	let purchasedAt = $state('');
	let openedAt = $state('');
	let emptiedAt = $state('');
	let weightGrams = $state<number | string>('');
	let diameterMm = $state<number | string>('');
	let notes = $state('');
	let errorMessage = $state<string | null>(null);

	$effect(() => {
		if (id === null) {
			listAllFilaments().then((f) => (filaments = f));
			return;
		}
		getSpool(id).then((existing) => {
			lotCode = existing.lotCode ?? '';
			purchasedAt = existing.purchasedAt ?? '';
			openedAt = existing.openedAt ?? '';
			emptiedAt = existing.emptiedAt ?? '';
			weightGrams = existing.weightGrams?.toString() ?? '';
			diameterMm = existing.diameterMm?.toString() ?? '';
			notes = existing.notes ?? '';
		});
	});

	async function save(e: Event) {
		e.preventDefault();
		const input: SpoolInput = {
			lotCode: lotCode.trim() || null,
			purchasedAt: purchasedAt || null,
			openedAt: openedAt || null,
			emptiedAt: emptiedAt || null,
			weightGrams: numOrNull(weightGrams),
			diameterMm: numOrNull(diameterMm),
			notes: notes.trim() || null
		};

		try {
			if (id !== null) {
				const result = await updateSpool(id, input);
				if (!result.ok) {
					errorMessage = result.error ?? 'Save failed.';
					return;
				}
				goto('/spools');
				return;
			}

			if (selectedFilamentId === 0) {
				errorMessage = 'Pick a filament.';
				return;
			}
			const result = await createSpool(selectedFilamentId, input);
			if (!result.ok) {
				errorMessage = result.error ?? 'Save failed.';
				return;
			}
			goto('/spools');
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<h1 class="mb-6 text-2xl font-semibold">{id === null ? 'Add spool' : 'Edit spool'}</h1>

<div class="max-w-lg space-y-4">
	{#if id === null}
		<div class="flex flex-col gap-1">
			<Label for="filament">Filament</Label>
			<FilamentPicker id="filament" bind:value={selectedFilamentId} {filaments} />
		</div>
	{/if}

	<form onsubmit={save} class="space-y-4">
		<div class="flex flex-col gap-1">
			<Label for="lot-code">Lot code</Label>
			<Input id="lot-code" bind:value={lotCode} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="purchased">Purchased</Label>
			<Input id="purchased" type="date" bind:value={purchasedAt} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="opened">Opened</Label>
			<Input id="opened" type="date" bind:value={openedAt} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="emptied">Emptied</Label>
			<Input id="emptied" type="date" bind:value={emptiedAt} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="weight">Weight (g)</Label>
			<Input id="weight" type="number" bind:value={weightGrams} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="diameter">Diameter (mm)</Label>
			<Input id="diameter" type="number" step="0.01" bind:value={diameterMm} />
		</div>
		<div class="flex flex-col gap-1">
			<Label for="notes">Notes</Label>
			<Textarea id="notes" bind:value={notes} class="min-h-[80px]" />
		</div>

		{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

		<div class="flex items-center gap-3 pt-2">
			<Button type="submit">Save</Button>
			<a href="/spools" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
		</div>
	</form>
</div>
