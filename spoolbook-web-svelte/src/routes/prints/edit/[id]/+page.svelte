<script lang="ts">
	import PrintForm from '$lib/components/prints/PrintForm.svelte';
	import { page } from '$app/state';

	let id = $derived(Number(page.params.id));
</script>

<svelte:head>
	<title>Edit print</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<PrintForm {id} />
</div>
