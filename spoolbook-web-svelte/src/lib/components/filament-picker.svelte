<script lang="ts">
	import SearchablePicker from '$lib/components/searchable-picker.svelte';
	import type { Filament } from '$lib/api/types';

	// Thin wrapper over SearchablePicker: builds the {value, label} option list from Filament
	// rows. The catalog runs into the thousands (brand/material/variant/color), so a plain
	// <Select> flattening every option into one scrolling list has no way to find one without
	// scrolling past hundreds of others first -- see SearchablePicker for the actual filtering.
	let {
		value = $bindable(),
		filaments,
		id,
		placeholder = '-- pick a filament --'
	}: {
		value: number;
		filaments: Filament[];
		id?: string;
		placeholder?: string;
	} = $props();

	let options = $derived(
		filaments.map((f) => ({ value: f.id, label: `${f.brand} ${f.material} ${f.variant ?? ''} — ${f.color}` }))
	);
</script>

<SearchablePicker {id} bind:value {options} {placeholder} searchPlaceholder="Search filaments…" />
