<script lang="ts">
	import PrinterCard from '$lib/components/printers/PrinterCard.svelte';
	import { listPrinters, me } from '$lib/api/client';
	import type { Printer } from '$lib/api/types';

	let printers = $state<Printer[] | null>(null);
	let authenticated = $state(false);

	async function load() {
		printers = await listPrinters();
	}

	$effect(() => {
		load();
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Printers</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<div class="mb-6 flex items-center justify-between">
		<h1 class="text-2xl font-semibold">Printers</h1>
		{#if authenticated}
			<a href="/printers/new" class="text-sm font-medium underline">+ Add printer</a>
		{/if}
	</div>

	{#if printers === null}
		<p class="text-muted-foreground">Loading…</p>
	{:else if printers.length === 0}
		<p class="text-muted-foreground">No printers yet.</p>
	{:else}
		<div class="space-y-6">
			{#each printers as p (p.id)}
				<PrinterCard printer={p} {authenticated} onDeleted={load} />
			{/each}
		</div>
	{/if}
</div>
