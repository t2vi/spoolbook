<script lang="ts">
	import * as Table from '$lib/components/ui/table/index.js';
	import { deleteSpool, listSpools, me } from '$lib/api/client';
	import type { Spool } from '$lib/api/types';

	let spools = $state<Spool[] | null>(null);
	let errorMessage = $state<string | null>(null);
	let authenticated = $state(false);

	async function load() {
		spools = await listSpools();
	}

	async function remove(id: number) {
		if (!authenticated) {
			errorMessage = 'Sign in to delete.';
			return;
		}
		const result = await deleteSpool(id);
		if (!result.ok) {
			errorMessage =
				result.error === 'has_profiles'
					? "Can't delete — a Print Profile references this spool."
					: result.error === 'has_prints'
						? "Can't delete — a Print references this spool."
						: (result.error ?? 'Delete failed.');
			return;
		}
		errorMessage = null;
		await load();
	}

	$effect(() => {
		load();
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Spools</title>
</svelte:head>

<div class="mx-auto max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Spools</h1>

	<div class="mb-4">
		{#if authenticated}
			<a href="/spools/new" class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/80">
				+ Add spool
			</a>
		{:else}
			<a href="/login" class="text-sm text-muted-foreground underline">Sign in to edit</a>
		{/if}
	</div>

	{#if spools === null}
		<p class="text-muted-foreground">Loading…</p>
	{:else}
		<div class="rounded-lg border bg-card shadow-sm">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Filament</Table.Head>
						<Table.Head>Lot code</Table.Head>
						<Table.Head>Purchased</Table.Head>
						<Table.Head>Opened</Table.Head>
						<Table.Head>Emptied</Table.Head>
						<Table.Head></Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each spools as s (s.id)}
						<Table.Row>
							<Table.Cell>{s.filament?.brand} {s.filament?.material} {s.filament?.variant ?? ''} — {s.filament?.color}</Table.Cell>
							<Table.Cell>{s.lotCode}</Table.Cell>
							<Table.Cell>{s.purchasedAt}</Table.Cell>
							<Table.Cell>{s.openedAt}</Table.Cell>
							<Table.Cell>{s.emptiedAt}</Table.Cell>
							<Table.Cell>
								{#if authenticated}
									<a href="/spools/edit/{s.id}" class="hover:underline">Edit</a>
									<button onclick={() => remove(s.id)} class="ml-3 text-destructive hover:underline">Delete</button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}

	{#if errorMessage}<p class="mt-3 text-sm text-destructive">{errorMessage}</p>{/if}
</div>
