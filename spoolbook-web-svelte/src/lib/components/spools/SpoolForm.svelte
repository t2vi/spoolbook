<script lang="ts">
	import FormModal from '$lib/components/form-modal.svelte';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import FilamentPicker from '$lib/components/filament-picker.svelte';
	import { createSpool, getSpool, listAllFilaments, updateSpool } from '$lib/api/client';
	import type { SpoolInput } from '$lib/api/client';
	import type { Filament } from '$lib/api/types';
	import { unsavedGuard } from '$lib/unsaved.svelte';
	import { numOrNull } from '$lib/utils.js';

	// `onClose(true)` after a save; `onClose(false)` to close without one.
	let { id, onClose }: { id: number | null; onClose: (saved: boolean) => void } = $props();

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
	const unsaved = unsavedGuard({
		form: () => [selectedFilamentId, lotCode, purchasedAt, openedAt, emptiedAt, weightGrams, diameterMm, notes]
	});

	$effect(() => {
		if (id === null) {
			listAllFilaments().then((f) => (filaments = f));
			unsaved.markClean();
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
			unsaved.markClean();
		});
	});

	async function save() {
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
				unsaved.markClean();
				onClose(true);
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
			unsaved.markClean();
			onClose(true);
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<FormModal title={id === null ? 'Add spool' : 'Edit spool'} error={errorMessage} onClose={() => onClose(false)} onSubmit={save}>
	{#if id === null}
		<div class="flex flex-col gap-1.5">
			<Label for="filament">Filament</Label>
			<FilamentPicker id="filament" bind:value={selectedFilamentId} {filaments} />
		</div>
	{/if}
	<div class="flex flex-col gap-1.5">
		<Label for="lot-code">Lot code</Label>
		<Input id="lot-code" bind:value={lotCode} />
	</div>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
		<div class="flex flex-col gap-1.5">
			<Label for="purchased">Purchased</Label>
			<Input id="purchased" type="date" bind:value={purchasedAt} />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="opened">Opened</Label>
			<Input id="opened" type="date" bind:value={openedAt} />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="emptied">Emptied</Label>
			<Input id="emptied" type="date" bind:value={emptiedAt} />
		</div>
	</div>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
		<div class="flex flex-col gap-1.5">
			<Label for="weight">Weight (g)</Label>
			<Input id="weight" type="number" bind:value={weightGrams} />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="diameter">Diameter (mm)</Label>
			<Input id="diameter" type="number" step="0.01" bind:value={diameterMm} />
		</div>
	</div>
	<div class="flex flex-col gap-1.5">
		<Label for="notes">Notes</Label>
		<Textarea id="notes" bind:value={notes} class="min-h-[80px]" />
	</div>
</FormModal>
