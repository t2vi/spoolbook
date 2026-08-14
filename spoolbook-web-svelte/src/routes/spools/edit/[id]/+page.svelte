<script lang="ts">
	import SpoolForm from '$lib/components/spools/SpoolForm.svelte';
	import { page } from '$app/state';

	let id = $derived(Number(page.params.id));
</script>

<svelte:head>
	<title>Edit spool</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<SpoolForm {id} />
</div>
