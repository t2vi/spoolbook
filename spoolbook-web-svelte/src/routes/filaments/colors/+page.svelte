<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Pagination from '$lib/components/ui/pagination/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import Picker from '$lib/components/picker.svelte';
	import { refreshColorHexes } from '$lib/color-hexes.svelte';
	import { createColor, deleteColor, listColorInventory, updateColor } from '$lib/api/client';
	import type { ColorInventoryResult } from '$lib/api/client';
	import type { FilamentColor } from '$lib/api/types';

	// Colors screen (007/FR-042): name filter, sortable name / hex, paging with a page-size choice,
	// add/edit in a dialog with a live swatch preview, delete with the in_use refusal shown.
	let result = $state<ColorInventoryResult | null>(null);
	let q = $state('');
	let sort = $state<'name' | 'hex'>('name');
	let order = $state<'asc' | 'desc'>('asc');
	let page = $state(1);
	let pageSize = $state(20);
	const pageSizes = [10, 20, 50].map((n) => ({ value: n, label: `${n} per page` }));

	async function load() {
		result = await listColorInventory(q, sort, order, page, pageSize);
	}

	function sortBy(column: 'name' | 'hex') {
		order = sort === column && order === 'asc' ? 'desc' : 'asc';
		sort = column;
		page = 1;
		load();
	}

	function arrow(column: 'name' | 'hex') {
		return sort === column ? (order === 'asc' ? ' ▲' : ' ▼') : '';
	}

	// Add / edit
	let editing = $state<{ id: number | null; name: string; hex: string } | null>(null);
	let formError = $state<string | null>(null);

	function openEditor(color: FilamentColor | null) {
		editing = color ? { id: color.id, name: color.name, hex: color.hex } : { id: null, name: '', hex: '#' };
		formError = null;
	}

	async function save() {
		if (!editing) return;
		const r = editing.id === null ? await createColor(editing.name, editing.hex) : await updateColor(editing.id, editing.name, editing.hex);
		if (!r.ok) {
			formError =
				r.error === 'duplicate'
					? 'A color with this name already exists.'
					: r.error === 'invalid_hex'
						? 'Each hex code must look like #RRGGBB (comma-separate for multi-color).'
						: (r.error ?? 'Save failed.');
			return;
		}
		editing = null;
		refreshColorHexes();
		await load();
	}

	// Delete
	let deleteTarget = $state<FilamentColor | null>(null);
	let deleteRefused = $state(false);

	async function confirmDelete() {
		if (!deleteTarget) return;
		const r = await deleteColor(deleteTarget.id);
		if (!r.ok && r.error === 'in_use') {
			deleteRefused = true;
			return;
		}
		deleteTarget = null;
		refreshColorHexes();
		await load();
	}

	function closeDelete() {
		deleteTarget = null;
		deleteRefused = false;
	}

	$effect(() => {
		load();
	});
</script>

<svelte:head>
	<title>Colors</title>
</svelte:head>

<div class="mx-auto max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Colors</h1>

	<div class="mb-4 flex flex-wrap items-center gap-3">
		<Input
			placeholder="Filter by name"
			bind:value={q}
			oninput={() => {
				page = 1;
				load();
			}}
			class="w-56"
		/>
		<Button onclick={() => openEditor(null)}>+ Add color</Button>
	</div>

	<div class="rounded-lg border bg-card shadow-sm">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head class="w-10"></Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('name')}>Name{arrow('name')}</button></Table.Head>
					<Table.Head><button class="hover:underline" onclick={() => sortBy('hex')}>Hex{arrow('hex')}</button></Table.Head>
					<Table.Head></Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if result === null}
					{#each Array(5) as _, i (i)}
						<Table.Row><Table.Cell colspan={4}><Skeleton class="h-5 w-full" /></Table.Cell></Table.Row>
					{/each}
				{:else if result.colors.length === 0}
					<Table.Row><Table.Cell colspan={4} class="text-center text-muted-foreground">No colors</Table.Cell></Table.Row>
				{:else}
					{#each result.colors as c (c.id)}
						<Table.Row>
							<Table.Cell><ColorSwatch hex={c.hex} class="size-6" /></Table.Cell>
							<Table.Cell>{c.name}</Table.Cell>
							<Table.Cell class="font-mono text-xs">{c.hex}</Table.Cell>
							<Table.Cell class="text-right">
								<button class="hover:underline" onclick={() => openEditor(c)}>Edit</button>
								<button class="ml-3 text-destructive hover:underline" onclick={() => (deleteTarget = c)}>Delete</button>
							</Table.Cell>
						</Table.Row>
					{/each}
				{/if}
			</Table.Body>
		</Table.Root>
	</div>

	{#if result}
		<div class="mt-4 flex flex-col items-center gap-2 text-sm text-muted-foreground">
			<div class="flex items-center gap-3">
				<span>Page {result.page} of {result.totalPages} ({result.total} total)</span>
				<div class="w-36">
					<Picker
						bind:value={pageSize}
						options={pageSizes}
						onValueChange={() => {
							page = 1;
							load();
						}}
					/>
				</div>
			</div>
			<Pagination.Root
				count={result.total}
				perPage={result.pageSize}
				{page}
				onPageChange={(p) => {
					page = p;
					load();
				}}
			>
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

{#if editing}
	<Dialog.Root open={true} onOpenChange={(open) => !open && (editing = null)}>
		<Dialog.Content class="max-w-md">
			<Dialog.Header>
				<Dialog.Title>{editing.id === null ? 'Add color' : 'Edit color'}</Dialog.Title>
			</Dialog.Header>
			<div class="space-y-4">
				<div class="flex flex-col gap-1">
					<Label for="color-name">Name</Label>
					<Input id="color-name" bind:value={editing.name} />
				</div>
				<div class="flex flex-col gap-1">
					<Label for="color-hex">Hex</Label>
					<div class="flex items-center gap-2">
						<Input id="color-hex" class="font-mono" bind:value={editing.hex} />
						<ColorSwatch hex={editing.hex} class="size-9" />
					</div>
					<p class="text-xs text-muted-foreground">#RRGGBB — comma-separate for multi-color, e.g. #1A1A1A, #D4AF37</p>
				</div>
				{#if formError}<p class="text-sm text-destructive">{formError}</p>{/if}
			</div>
			<Dialog.Footer>
				<Button variant="ghost" onclick={() => (editing = null)}>Cancel</Button>
				<Button onclick={save}>Save</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>
{/if}

<AlertDialog.Root open={deleteTarget !== null} onOpenChange={(open) => !open && closeDelete()}>
	<AlertDialog.Content class="sm:max-w-md">
		{#if deleteRefused}
			<AlertDialog.Header>
				<AlertDialog.Title>Can't delete</AlertDialog.Title>
				<AlertDialog.Description>Filaments still use “{deleteTarget?.name}”. Change their color first.</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer><AlertDialog.Action onclick={closeDelete}>OK</AlertDialog.Action></AlertDialog.Footer>
		{:else}
			<AlertDialog.Header>
				<AlertDialog.Title>Delete “{deleteTarget?.name}”?</AlertDialog.Title>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel onclick={closeDelete}>Cancel</AlertDialog.Cancel>
				<AlertDialog.Action onclick={confirmDelete}>Delete</AlertDialog.Action>
			</AlertDialog.Footer>
		{/if}
	</AlertDialog.Content>
</AlertDialog.Root>
