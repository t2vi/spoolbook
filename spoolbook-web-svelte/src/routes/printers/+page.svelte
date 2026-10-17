<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import PrinterCard from '$lib/components/printers/PrinterCard.svelte';
	import { listPrinters, me } from '$lib/api/client';
	import type { Printer } from '$lib/api/types';
	import PrinterForm from '$lib/components/printers/PrinterForm.svelte';
	import { formHref, openForm } from '$lib/form-url';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let printers = $state<Printer[] | null>(null);
	let authenticated = $state(false);

	// The open add/edit modal lives in the URL (002/FR-015); closing drops it, which runs the
	// unsaved-changes guard, and keeps the list's own query.
	let form = $derived(openForm(page.url));
	async function closeForm(saved: boolean) {
		await goto(formHref(page.url, undefined), { noScroll: true });
		if (saved) await load();
	}

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

<div class="mx-auto w-full max-w-3xl px-4 py-8">
	<div class="mb-6 flex items-center justify-between">
		<h1 class="text-2xl font-semibold">Printers</h1>
		{#if authenticated}
			<Button href={formHref(page.url, null)}>+ Add printer</Button>
		{/if}
	</div>

	{#if printers === null}
		<div class="space-y-6">
			{#each Array(2) as _, i (i)}
				<div class="rounded-lg border bg-card p-6 shadow-sm">
					<div class="flex items-center gap-4">
						<Skeleton class="h-20 w-20 rounded-md" />
						<div class="space-y-2">
							<Skeleton class="h-5 w-32" />
							<Skeleton class="h-4 w-16" />
						</div>
					</div>
					<Skeleton class="mt-6 h-24 w-full" />
				</div>
			{/each}
		</div>
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

{#if form !== undefined}
	{#key form}
		<PrinterForm id={form} onClose={closeForm} />
	{/key}
{/if}
