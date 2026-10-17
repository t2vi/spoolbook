<script lang="ts">
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { hexFor } from '$lib/color-hexes.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import Picker from '$lib/components/picker.svelte';
	import { deleteProfile, listProfileInventory, listSpools, me } from '$lib/api/client';
	import type { ProfileInventoryResult, Spool } from '$lib/api/types';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ProfileForm from '$lib/components/profiles/ProfileForm.svelte';
	import { formHref, listParams, openForm } from '$lib/form-url';

	// Profiles list (006/FR-040): the selected version of each profile. Filters, sort and page
	// live in the URL, so a view survives refresh, back and sharing; the server does the
	// filtering (006/FR-002).
	let result = $state<ProfileInventoryResult | null>(null);
	let errorMessage = $state<string | null>(null);
	let authenticated = $state(false);
	// Brand / material choices: the ones you actually own spools of, not the whole catalog.
	let owned = $state<Spool[]>([]);

	const param = (key: string) => page.url.searchParams.get(key) ?? '';
	let sort = $derived(param('sort') || 'filament');
	let order = $derived(param('order') || 'asc');
	let pageSize = $derived(Number(param('pageSize')) || 20);

	const distinct = (values: (string | undefined)[]) => [...new Set(values.filter((v): v is string => !!v))].sort();
	let brandOptions = $derived([{ value: '', label: 'All brands' }, ...distinct(owned.map((s) => s.filament?.brand)).map((b) => ({ value: b, label: b }))]);
	let materialOptions = $derived([
		{ value: '', label: 'All materials' },
		...distinct(owned.map((s) => s.filament?.material)).map((m) => ({ value: m, label: m }))
	]);
	const scopeOptions = [
		{ value: '', label: 'Any scope' },
		{ value: 'generic', label: 'Generic' },
		{ value: 'spool', label: 'Spool-specific' }
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
		setQuery({ sort: column, order: sort === column && order === 'asc' ? 'desc' : 'asc' });
	}

	function arrow(column: string) {
		return sort === column ? (order === 'asc' ? ' ▲' : ' ▼') : '';
	}

	// The list reloads on every URL change, closing a form included.
	let form = $derived(openForm(page.url));
	function closeForm() {
		goto(formHref(page.url, undefined), { noScroll: true });
	}

	async function load() {
		result = await listProfileInventory(listParams(page.url));
	}

	async function remove(id: number) {
		const r = await deleteProfile(id);
		if (!r.ok) {
			errorMessage =
				r.error === 'has_prints'
					? "Can't delete — a Print used this profile version."
					: r.error === 'has_newer_version'
						? "Can't delete — a newer version builds on this one."
						: (r.error ?? 'Delete failed.');
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
		listSpools().then((s) => (owned = s));
	});
</script>

<svelte:head>
	<title>Print Profiles</title>
</svelte:head>

<div class="mx-auto w-full max-w-6xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Print Profiles</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<div class="w-40"><Picker value={param('brand')} options={brandOptions} onValueChange={(v) => setQuery({ brand: v })} /></div>
		<div class="w-40"><Picker value={param('material')} options={materialOptions} onValueChange={(v) => setQuery({ material: v })} /></div>
		<div class="w-40"><Picker value={param('scope')} options={scopeOptions} onValueChange={(v) => setQuery({ scope: v })} /></div>
		{#if page.url.search}<Button variant="ghost" onclick={() => goto('?')}>Clear</Button>{/if}
		<div class="ml-auto">
			{#if authenticated}<Button href={formHref(page.url, null)}>+ Add profile</Button>{/if}
		</div>
	</div>

	<div class="rounded-lg border bg-card shadow-sm">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('name')}>Name{arrow('name')}</button></Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('filament')}>Filament{arrow('filament')}</button></Table.Head>
					<Table.Head>Scope</Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('nozzleTemp')}>Nozzle{arrow('nozzleTemp')}</button></Table.Head>
					<Table.Head>Source</Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('created')}>Version{arrow('created')}</button></Table.Head>
					<Table.Head></Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if result === null}
					{#each Array(5) as _, i (i)}
						<Table.Row><Table.Cell colspan={7}><Skeleton class="h-5 w-full" /></Table.Cell></Table.Row>
					{/each}
				{:else if result.profiles.length === 0}
					<Table.Row><Table.Cell colspan={7} class="text-center text-muted-foreground">No profiles</Table.Cell></Table.Row>
				{:else}
					{#each result.profiles as p (p.id)}
						<Table.Row>
							<Table.Cell>{p.name}</Table.Cell>
							<Table.Cell><span class="flex items-center gap-2"><ColorSwatch hex={hexFor(p.filament?.color)} />{p.filament?.brand} {p.filament?.material} {p.filament?.variant ?? ''} — {p.filament?.color}</span></Table.Cell>
							<Table.Cell>{p.spoolId ? (p.spool?.lotCode ?? `Spool #${p.spoolId}`) : 'Generic'}</Table.Cell>
							<Table.Cell>{p.nozzleTempC}°C</Table.Cell>
							<Table.Cell>{p.source}</Table.Cell>
							<Table.Cell>v{p.versionNumber}{p.versionName ? ` · ${p.versionName}` : ''}</Table.Cell>
							<Table.Cell class="text-right whitespace-nowrap">
								{#if authenticated}
									<a href={formHref(page.url, p.id)} class="hover:underline">Edit</a>
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

{#if form !== undefined}
	{#key form}
		<ProfileForm id={form} onClose={closeForm} />
	{/key}
{/if}
