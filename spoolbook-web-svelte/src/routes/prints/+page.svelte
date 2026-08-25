<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import Picker from '$lib/components/picker.svelte';
	import { deletePrint, me, searchPrints } from '$lib/api/client';
	import type { PrintInventoryResult, PrintStatus } from '$lib/api/types';
	import { page } from '$app/state';

	let printerId = $derived(page.url.searchParams.get('printerId') ? Number(page.url.searchParams.get('printerId')) : null);

	let result = $state<PrintInventoryResult | null>(null);
	let statusFilter = $state<PrintStatus | ''>('');
	let pageNum = $state(1);
	let errorMessage = $state<string | null>(null);
	let authenticated = $state(false);

	async function load() {
		result = await searchPrints(statusFilter, printerId, pageNum);
	}

	function applyFilters() {
		pageNum = 1;
		load();
	}

	function changePage(newPage: number) {
		pageNum = newPage;
		load();
	}

	async function remove(id: number) {
		if (!authenticated) {
			errorMessage = 'Sign in to delete.';
			return;
		}
		const result = await deletePrint(id);
		if (!result.ok) {
			errorMessage = result.error ?? 'Delete failed.';
			return;
		}
		errorMessage = null;
		await load();
	}

	function statusBadgeClass(status: PrintStatus) {
		switch (status) {
			case 'Success':
				return 'bg-green-100 text-green-800 hover:bg-green-100';
			case 'Partial':
				return 'bg-amber-100 text-amber-800 hover:bg-amber-100';
			case 'Failed':
				return 'bg-red-100 text-red-800 hover:bg-red-100';
			case 'InProgress':
				return 'bg-blue-100 text-blue-800 hover:bg-blue-100';
		}
	}

	$effect(() => {
		load();
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Prints</title>
</svelte:head>

<div class="mx-auto max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Prints</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<Picker
			bind:value={statusFilter}
			onValueChange={applyFilters}
			options={[
				{ value: '', label: 'All statuses' },
				{ value: 'Success', label: 'Success' },
				{ value: 'Failed', label: 'Failed' },
				{ value: 'Partial', label: 'Partial' },
				{ value: 'InProgress', label: 'InProgress' }
			]}
		/>
		{#if printerId !== null}
			<span class="text-sm text-muted-foreground">Filtered by printer</span>
			<a href="/prints" class="text-sm text-muted-foreground underline">Clear</a>
		{/if}
	</div>

	{#if result === null}
		<p class="text-muted-foreground">Loading…</p>
	{:else}
		<div class="rounded-lg border bg-card shadow-sm">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Started</Table.Head>
						<Table.Head>Filament</Table.Head>
						<Table.Head>Profile</Table.Head>
						<Table.Head>Printer</Table.Head>
						<Table.Head>Status</Table.Head>
						<Table.Head></Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.prints as p (p.id)}
						<Table.Row>
							<Table.Cell>{new Date(p.startedAt).toLocaleString()}</Table.Cell>
							<Table.Cell>{p.spool?.filament?.brand} {p.spool?.filament?.material} — {p.spool?.filament?.color}</Table.Cell>
							<Table.Cell>{p.profile?.name}</Table.Cell>
							<Table.Cell>{p.printer?.name}</Table.Cell>
							<Table.Cell><Badge class={statusBadgeClass(p.status)}>{p.status}</Badge></Table.Cell>
							<Table.Cell>
								<a href="/prints/{p.id}" class="hover:underline">View</a>
								{#if authenticated}
									<button onclick={() => remove(p.id)} class="ml-3 text-destructive hover:underline">Delete</button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>

		<div class="mt-4 flex flex-col items-center gap-2 text-sm text-muted-foreground">
			<span>Page {result.page} of {result.totalPages} ({result.total} total)</span>
			<Pagination.Root count={result.total} perPage={result.pageSize} page={pageNum} onPageChange={changePage}>
				{#snippet children({ pages, currentPage })}
					<Pagination.Content>
						<Pagination.Item>
							<Pagination.PrevButton />
						</Pagination.Item>
						{#each pages as p (p.key)}
							{#if p.type === 'ellipsis'}
								<Pagination.Item><Pagination.Ellipsis /></Pagination.Item>
							{:else}
								<Pagination.Item>
									<Pagination.Link page={p} isActive={currentPage === p.value}>{p.value}</Pagination.Link>
								</Pagination.Item>
							{/if}
						{/each}
						<Pagination.Item>
							<Pagination.NextButton />
						</Pagination.Item>
					</Pagination.Content>
				{/snippet}
			</Pagination.Root>
		</div>
	{/if}

	{#if errorMessage}<p class="mt-3 text-sm text-destructive">{errorMessage}</p>{/if}
</div>
