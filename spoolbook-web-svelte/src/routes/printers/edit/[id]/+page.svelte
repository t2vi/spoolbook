<script lang="ts">
	import PrinterForm from '$lib/components/printers/PrinterForm.svelte';
	import { page } from '$app/state';

	let id = $derived(Number(page.params.id));
</script>

<svelte:head>
	<title>Edit printer</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<PrinterForm {id} />
</div>
