<script lang="ts">
	import FilamentForm from '$lib/components/filaments/FilamentForm.svelte';
	import { page } from '$app/state';

	let id = $derived(Number(page.params.id));
</script>

<svelte:head>
	<title>Edit filament</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<FilamentForm {id} />
</div>
