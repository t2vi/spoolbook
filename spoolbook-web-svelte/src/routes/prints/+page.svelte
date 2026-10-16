<script lang="ts">
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { hexFor } from '$lib/color-hexes.svelte';
	import { Badge, printStatusVariant } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import Picker from '$lib/components/picker.svelte';
	import { deletePrint, listPrintInventory, listPrinters, listSpools, me } from '$lib/api/client';
	import type { PrintInventoryResult, PrintStatus, Printer, Spool } from '$lib/api/types';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { formatDate } from '$lib/utils.js';

	// Prints list (003/FR-050). Filters, sort and page live in the URL, so a view survives refresh,
	// back and sharing; the server does the filtering (003/FR-003).
	let result = $state<PrintInventoryResult | null>(null);
	let errorMessage = $state<string | null>(null);
	let authenticated = $state(false);
	let printers = $state<Printer[]>([]);
	// Brand / material choices: the ones you actually own spools of, not the whole catalog.
	let owned = $state<Spool[]>([]);

	const param = (key: string) => page.url.searchParams.get(key) ?? '';
	const defaultOrder = (column: string) => (column === 'startedAt' ? 'desc' : 'asc');
	let sort = $derived(param('sort') || 'startedAt');
	let order = $derived(param('order') || defaultOrder(sort));
	let pageSize = $derived(Number(param('pageSize')) || 20);

	const distinct = (values: (string | undefined)[]) => [...new Set(values.filter((v): v is string => !!v))].sort();
	let brandOptions = $derived([{ value: '', label: 'All brands' }, ...distinct(owned.map((s) => s.filament?.brand)).map((b) => ({ value: b, label: b }))]);
	let materialOptions = $derived([
		{ value: '', label: 'All materials' },
		...distinct(owned.map((s) => s.filament?.material)).map((m) => ({ value: m, label: m }))
	]);
	let printerOptions = $derived([{ value: '', label: 'All printers' }, ...printers.map((p) => ({ value: String(p.id), label: p.name }))]);
	const statusOptions = [
		{ value: '', label: 'All statuses' },
		{ value: 'Success', label: 'Success' },
		{ value: 'Failed', label: 'Failed' },
		{ value: 'Partial', label: 'Partial' },
		{ value: 'InProgress', label: 'InProgress' }
	];
	const pageSizes = [10, 20, 50].map((n) => ({ value: n, label: `${n} per page` }));

	// Any change but paging itself goes back to page 1.
	function setQuery(changes: Record<string, string | number | null>) {
		const query = new URLSearchParams(page.url.searchParams);
		if (!('page' in changes)) query.delete('page');
		for (const [key, value] of Object.entries(changes)) {
			if (value === null || value === '') query.delete(key);
			else query.set(key, String(value));
		}
		goto(`?${query}`, { keepFocus: true, noScroll: true });
	}

	function sortBy(column: string) {
		setQuery({ sort: column, order: sort === column ? (order === 'asc' ? 'desc' : 'asc') : defaultOrder(column) });
	}

	function arrow(column: string) {
		return sort === column ? (order === 'asc' ? ' ▲' : ' ▼') : '';
	}

	async function load() {
		result = await listPrintInventory(page.url.searchParams);
	}

	async function remove(id: number) {
		const r = await deletePrint(id);
		if (!r.ok) {
			errorMessage = r.error ?? 'Delete failed.';
			return;
		}
		errorMessage = null;
		await load();
	}

	$effect(() => {
		load();
	});

	$effect(() => {
		me().then((r) => (authenticated = r.authenticated));
		listPrinters().then((p) => (printers = p));
		listSpools().then((s) => (owned = s));
	});
</script>

<svelte:head>
	<title>Prints</title>
</svelte:head>

<div class="mx-auto w-full max-w-6xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Prints</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<div class="w-40"><Picker value={param('brand')} options={brandOptions} onValueChange={(v) => setQuery({ brand: v })} /></div>
		<div class="w-40"><Picker value={param('material')} options={materialOptions} onValueChange={(v) => setQuery({ material: v })} /></div>
		<div class="w-40"><Picker value={param('printerId')} options={printerOptions} onValueChange={(v) => setQuery({ printerId: v })} /></div>
		<div class="w-40"><Picker value={param('status')} options={statusOptions} onValueChange={(v) => setQuery({ status: v })} /></div>
		{#if page.url.search}<Button variant="ghost" onclick={() => goto('?')}>Clear</Button>{/if}
	</div>

	<div class="rounded-lg border bg-card shadow-sm">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('startedAt')}>Started{arrow('startedAt')}</button></Table.Head>
					<Table.Head>Filament</Table.Head>
					<Table.Head>Profile</Table.Head>
					<Table.Head>Spool</Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('printer')}>Printer{arrow('printer')}</button></Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('status')}>Status{arrow('status')}</button></Table.Head>
					<Table.Head>Ambient</Table.Head>
					<Table.Head></Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if result === null}
					{#each Array(5) as _, i (i)}
						<Table.Row><Table.Cell colspan={8}><Skeleton class="h-5 w-full" /></Table.Cell></Table.Row>
					{/each}
				{:else if result.prints.length === 0}
					<Table.Row><Table.Cell colspan={8} class="text-center text-muted-foreground">No prints</Table.Cell></Table.Row>
				{:else}
					{#each result.prints as p (p.id)}
						<Table.Row>
							<Table.Cell>{formatDate(p.startedAt)}</Table.Cell>
							<Table.Cell><span class="flex items-center gap-2"><ColorSwatch hex={hexFor(p.spool?.filament?.color)} />{p.spool?.filament?.brand} {p.spool?.filament?.material} — {p.spool?.filament?.color}</span></Table.Cell>
							<Table.Cell>{p.profile?.name}</Table.Cell>
							<Table.Cell>{p.spool?.lotCode ?? ''}</Table.Cell>
							<Table.Cell>{p.printer?.name}</Table.Cell>
							<Table.Cell><Badge variant={printStatusVariant[p.status]}>{p.status}</Badge></Table.Cell>
							<Table.Cell>{p.ambientTempC !== null ? `${p.ambientTempC.toFixed(1)} °C` : ''}</Table.Cell>
							<Table.Cell class="text-right whitespace-nowrap">
								<a href="/prints/{p.id}" class="hover:underline">View</a>
								{#if authenticated}
									<button onclick={() => remove(p.id)} class="ml-3 text-destructive hover:underline">Delete</button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				{/if}
			</Table.Body>
		</Table.Root>
	</div>

	{#if errorMessage}<p class="mt-3 text-sm text-destructive">{errorMessage}</p>{/if}

	{#if result}
		<div class="mt-4 flex flex-col items-center gap-2 text-sm text-muted-foreground">
			<div class="flex items-center gap-3">
				<span>Page {result.page} of {result.totalPages} ({result.total} total)</span>
				<div class="w-36"><Picker value={pageSize} options={pageSizes} onValueChange={(n) => setQuery({ pageSize: n })} /></div>
			</div>
			<Pagination.Root count={result.total} perPage={result.pageSize} page={result.page} onPageChange={(p) => setQuery({ page: p })}>
				{#snippet children({ pages, currentPage })}
					<Pagination.Content>
						<Pagination.Item><Pagination.PrevButton /></Pagination.Item>
						{#each pages as p (p.key)}
							{#if p.type === 'ellipsis'}
								<Pagination.Item><Pagination.Ellipsis /></Pagination.Item>
							{:else}
								<Pagination.Item><Pagination.Link page={p} isActive={currentPage === p.value}>{p.value}</Pagination.Link></Pagination.Item>
							{/if}
						{/each}
						<Pagination.Item><Pagination.NextButton /></Pagination.Item>
					</Pagination.Content>
				{/snippet}
			</Pagination.Root>
		</div>
	{/if}
</div>
