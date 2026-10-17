<script lang="ts">
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { hexFor } from '$lib/color-hexes.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import { deleteFilament, me, searchFilaments, syncFilamentCatalog } from '$lib/api/client';
	import type { FilamentSearchResult } from '$lib/api/types';
	import FilamentForm from '$lib/components/filaments/FilamentForm.svelte';
	import { formHref, openForm } from '$lib/form-url';
	import { goto } from '$app/navigation';
	import { page as appPage } from '$app/state';

	let result = $state<FilamentSearchResult | null>(null);
	let brandFilter = $state('');
	let materialFilter = $state('');
	let page = $state(1);
	let errorMessage = $state<string | null>(null);
	let syncing = $state(false);
	let syncStatusMessage = $state<string | null>(null);
	let authenticated = $state(false);

	// The open add/edit modal lives in the URL (002/FR-015); closing drops it, which runs the
	// unsaved-changes guard, and keeps the list's own query.
	let form = $derived(openForm(appPage.url));
	async function closeForm(saved: boolean) {
		await goto(formHref(appPage.url, undefined), { noScroll: true });
		if (saved) await load();
	}

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
			errorMessage =
				deleteResult.error === 'has_spools'
					? "Can't delete — spools reference this filament."
					: deleteResult.error === 'has_profiles'
						? "Can't delete — print profiles reference this filament."
						: (deleteResult.error ?? 'Delete failed.');
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
		} catch (e) {
			// A failed sync is a 4xx, which request() throws on.
			syncStatusMessage = `Sync failed: ${e instanceof Error ? e.message : 'unknown error'}`;
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

<div class="mx-auto w-full max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Filaments</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<Input placeholder="Filter by brand" bind:value={brandFilter} oninput={applyFilters} class="w-48" />
		<Input placeholder="Filter by material" bind:value={materialFilter} oninput={applyFilters} class="w-48" />
		{#if authenticated}
			<Button href={formHref(appPage.url, null)}>+ Add filament</Button>
			<Button variant="outline" disabled={syncing} onclick={syncCatalog}>{syncing ? 'Syncing…' : 'Sync filament catalog'}</Button>
		{:else}
			<a href="/login" class="text-sm text-muted-foreground underline">Sign in to edit</a>
		{/if}
	</div>

	{#if syncStatusMessage}<p class="mb-3 text-sm text-muted-foreground">{syncStatusMessage}</p>{/if}

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
				{#if result === null}
					{#each Array(5) as _, i (i)}
						<Table.Row>
							<Table.Cell colspan={5}><Skeleton class="h-5 w-full" /></Table.Cell>
						</Table.Row>
					{/each}
				{:else}
					{#each result.entries as f (f.id)}
						<Table.Row>
							<Table.Cell>{f.brand}</Table.Cell>
							<Table.Cell>{f.material}</Table.Cell>
							<Table.Cell>{f.variant}</Table.Cell>
							<Table.Cell><span class="flex items-center gap-2"><ColorSwatch hex={hexFor(f.color)} />{f.color}</span></Table.Cell>
							<Table.Cell>
								{#if authenticated}
									<a href={formHref(appPage.url, f.id)} class="hover:underline">Edit</a>
									<button onclick={() => remove(f.id)} class="ml-3 text-destructive hover:underline">Delete</button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				{/if}
			</Table.Body>
		</Table.Root>
	</div>

	{#if result}
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

{#if form !== undefined}
	{#key form}
		<FilamentForm id={form} onClose={closeForm} />
	{/key}
{/if}
