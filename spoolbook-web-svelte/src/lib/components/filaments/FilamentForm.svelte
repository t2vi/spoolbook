<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { createFilament, listAllFilaments, listFilamentColors, updateFilament } from '$lib/api/client';
	import type { FilamentInput } from '$lib/api/client';
	import type { FilamentColor } from '$lib/api/types';
	import { goto } from '$app/navigation';

	let { id }: { id: number | null } = $props();

	let brand = $state('');
	let material = $state('');
	let variant = $state('');
	let color = $state('');
	let colors = $state<FilamentColor[]>([]);
	let errorMessage = $state<string | null>(null);

	let swatchHex = $derived(colors.find((c) => c.name === color)?.hex ?? null);

	$effect(() => {
		listFilamentColors().then((c) => (colors = c));

		if (id === null) return;
		listAllFilaments().then((all) => {
			const existing = all.find((f) => f.id === id);
			if (!existing) return;
			brand = existing.brand;
			material = existing.material;
			variant = existing.variant ?? '';
			color = existing.color;
		});
	});

	async function save(e: Event) {
		e.preventDefault();
		const input: FilamentInput = { brand, material, variant: variant.trim() || null, color };
		try {
			const result = id === null ? await createFilament(input) : await updateFilament(id, input);
			if (!result.ok) {
				errorMessage = result.error === 'duplicate' ? 'This filament already exists.' : (result.error ?? 'Save failed.');
				return;
			}
			goto('/filaments');
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<h1 class="mb-6 text-2xl font-semibold">{id === null ? 'Add filament' : 'Edit filament'}</h1>

<form onsubmit={save} class="max-w-lg space-y-4">
	<div class="flex flex-col gap-1">
		<Label for="brand">Brand</Label>
		<Input id="brand" bind:value={brand} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="material">Material</Label>
		<Input id="material" bind:value={material} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="variant">Variant</Label>
		<Input id="variant" bind:value={variant} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="color">Color</Label>
		<div class="flex items-center gap-2">
			<Input id="color" bind:value={color} list="color-options" />
			<datalist id="color-options">
				{#each colors as c (c.id)}
					<option value={c.name}></option>
				{/each}
			</datalist>
			{#if swatchHex}
				<span class="inline-block h-5 w-5 shrink-0 rounded border" style:background={swatchHex}></span>
			{/if}
		</div>
	</div>

	{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

	<div class="flex items-center gap-3 pt-2">
		<Button type="submit">Save</Button>
		<a href="/filaments" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
	</div>
</form>
