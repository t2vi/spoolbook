<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import { deleteFilament, me, searchFilaments, syncFilamentCatalog } from '$lib/api/client';
	import type { FilamentSearchResult } from '$lib/api/types';

	let result = $state<FilamentSearchResult | null>(null);
	let brandFilter = $state('');
	let materialFilter = $state('');
	let page = $state(1);
	let errorMessage = $state<string | null>(null);
	let syncing = $state(false);
	let syncStatusMessage = $state<string | null>(null);
	let authenticated = $state(false);

	async function load() {
		result = await searchFilaments(brandFilter, materialFilter, page);
	}

	function applyFilters() {
		page = 1;
		load();
	}

	function changePage(newPage: number) {
		page = newPage;
		load();
	}

	async function remove(id: number) {
		if (!authenticated) {
			errorMessage = 'Sign in to delete.';
			return;
		}
		const deleteResult = await deleteFilament(id);
		if (!deleteResult.ok) {
			errorMessage = deleteResult.error === 'has_spools' ? "Can't delete — spools reference this filament." : (deleteResult.error ?? 'Delete failed.');
			return;
		}
		errorMessage = null;
		await load();
	}

	async function syncCatalog() {
		syncing = true;
		syncStatusMessage = 'Syncing filament catalog…';
		try {
			const r = await syncFilamentCatalog();
			syncStatusMessage = r.ok ? `Added ${r.added} new, skipped ${r.skipped} duplicates.` : `Sync failed: ${r.error}`;
			if (r.ok) await load();
		} finally {
			syncing = false;
		}
	}

	$effect(() => {
		load();
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Filaments</title>
</svelte:head>

<div class="mx-auto max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Filaments</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<Input placeholder="Filter by brand" bind:value={brandFilter} oninput={applyFilters} class="w-48" />
		<Input placeholder="Filter by material" bind:value={materialFilter} oninput={applyFilters} class="w-48" />
		{#if authenticated}
			<a href="/filaments/new" class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/80">
				+ Add filament
			</a>
			<Button variant="outline" disabled={syncing} onclick={syncCatalog}>{syncing ? 'Syncing…' : 'Sync filament catalog'}</Button>
		{:else}
			<a href="/login" class="text-sm text-muted-foreground underline">Sign in to edit</a>
		{/if}
	</div>

	{#if syncStatusMessage}<p class="mb-3 text-sm text-muted-foreground">{syncStatusMessage}</p>{/if}

	{#if result === null}
		<p class="text-muted-foreground">Loading…</p>
	{:else}
		<div class="rounded-lg border bg-card shadow-sm">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Brand</Table.Head>
						<Table.Head>Material</Table.Head>
						<Table.Head>Variant</Table.Head>
						<Table.Head>Color</Table.Head>
						<Table.Head></Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.entries as f (f.id)}
						<Table.Row>
							<Table.Cell>{f.brand}</Table.Cell>
							<Table.Cell>{f.material}</Table.Cell>
							<Table.Cell>{f.variant}</Table.Cell>
							<Table.Cell>{f.color}</Table.Cell>
							<Table.Cell>
								{#if authenticated}
									<a href="/filaments/edit/{f.id}" class="hover:underline">Edit</a>
									<button onclick={() => remove(f.id)} class="ml-3 text-destructive hover:underline">Delete</button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>

		<div class="mt-4 flex flex-col items-center gap-2 text-sm text-muted-foreground">
			<span>Page {result.page} of {result.totalPages} ({result.total} total)</span>
			<Pagination.Root count={result.total} perPage={result.pageSize} page={page} onPageChange={changePage}>
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
