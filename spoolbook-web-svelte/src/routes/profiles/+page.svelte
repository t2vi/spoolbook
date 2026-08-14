<script lang="ts">
	import * as Table from '$lib/components/ui/table/index.js';
	import { deleteProfile, me, searchProfiles } from '$lib/api/client';
	import type { ProfileInventoryResult } from '$lib/api/types';

	let result = $state<ProfileInventoryResult | null>(null);
	let errorMessage = $state<string | null>(null);
	let authenticated = $state(false);

	async function load() {
		result = await searchProfiles();
	}

	async function remove(id: number) {
		if (!authenticated) {
			errorMessage = 'Sign in to delete.';
			return;
		}
		const result = await deleteProfile(id);
		if (!result.ok) {
			errorMessage = result.error === 'has_prints' ? "Can't delete — a Print references this profile version." : (result.error ?? 'Delete failed.');
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
	<title>Print Profiles</title>
</svelte:head>

<div class="mx-auto max-w-5xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Print Profiles</h1>

	<div class="mb-4">
		{#if authenticated}
			<a href="/profiles/new" class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/80">
				+ Add profile
			</a>
		{:else}
			<a href="/login" class="text-sm text-muted-foreground underline">Sign in to edit</a>
		{/if}
	</div>

	{#if result === null}
		<p class="text-muted-foreground">Loading…</p>
	{:else}
		<div class="rounded-lg border bg-card shadow-sm">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Name</Table.Head>
						<Table.Head>Filament</Table.Head>
						<Table.Head>Nozzle temp</Table.Head>
						<Table.Head>Source</Table.Head>
						<Table.Head></Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.profiles as p (p.id)}
						<Table.Row>
							<Table.Cell>{p.name}</Table.Cell>
							<Table.Cell>{p.filament?.brand} {p.filament?.material} {p.filament?.variant ?? ''} — {p.filament?.color}</Table.Cell>
							<Table.Cell>{p.nozzleTempC}°C</Table.Cell>
							<Table.Cell>{p.source}</Table.Cell>
							<Table.Cell>
								{#if authenticated}
									<a href="/profiles/edit/{p.id}" class="hover:underline">Edit</a>
									<button onclick={() => remove(p.id)} class="ml-3 text-destructive hover:underline">Delete</button>
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
