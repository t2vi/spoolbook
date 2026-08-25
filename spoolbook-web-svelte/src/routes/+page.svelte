<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import { getDashboard } from '$lib/api/client';
	import type { DashboardSnapshot } from '$lib/api/types';
	import { formatDateTime } from '$lib/utils.js';

	let snapshot = $state<DashboardSnapshot | null>(null);

	$effect(() => {
		getDashboard().then((s) => (snapshot = s));
	});
</script>

<svelte:head>
	<title>Spoolbook</title>
</svelte:head>

<div class="mx-auto max-w-6xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Dashboard</h1>

	{#if snapshot === null}
		<div class="mb-8 grid grid-cols-1 gap-4 sm:grid-cols-4">
			{#each Array(4) as _, i (i)}
				<div class="rounded-lg border bg-card p-5 shadow-sm">
					<Skeleton class="h-8 w-12" />
					<Skeleton class="mt-2 h-4 w-20" />
				</div>
			{/each}
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
			{#each Array(4) as _, i (i)}
				<Card.Root class="p-5">
					<Skeleton class="mb-3 h-4 w-32" />
					<div class="space-y-2">
						<Skeleton class="h-4 w-full" />
						<Skeleton class="h-4 w-full" />
						<Skeleton class="h-4 w-full" />
					</div>
				</Card.Root>
			{/each}
		</div>
	{:else}
		{@const metrics = snapshot.metrics}
		<div class="mb-8 grid grid-cols-1 gap-4 sm:grid-cols-4">
			<a href="/filaments" class="rounded-lg border bg-card p-5 shadow-sm transition hover:shadow">
				<div class="text-3xl font-semibold">{metrics.filamentCount}</div>
				<div class="text-sm text-muted-foreground">Filaments</div>
			</a>
			<a href="/spools" class="rounded-lg border bg-card p-5 shadow-sm transition hover:shadow">
				<div class="text-3xl font-semibold">{metrics.spoolsByStatus.reduce((sum, c) => sum + c.count, 0)}</div>
				<div class="text-sm text-muted-foreground">Spools</div>
			</a>
			<a href="/profiles" class="rounded-lg border bg-card p-5 shadow-sm transition hover:shadow">
				<div class="text-3xl font-semibold">{snapshot.profileCount}</div>
				<div class="text-sm text-muted-foreground">Print Profiles</div>
			</a>
			<a href="/prints" class="rounded-lg border bg-card p-5 shadow-sm transition hover:shadow">
				<div class="text-3xl font-semibold">{metrics.printsByStatus.reduce((sum, c) => sum + c.count, 0)}</div>
				<div class="text-sm text-muted-foreground">Prints logged</div>
			</a>
		</div>

		{#if metrics.lastFilamentSyncAt}
			<p class="mb-6 text-sm text-muted-foreground">
				Filament catalog last synced {formatDateTime(metrics.lastFilamentSyncAt)}
			</p>
		{/if}

		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
			<Card.Root class="p-5">
				<h3 class="mb-3 text-sm font-semibold">Spools by status</h3>
				<ul class="space-y-1 text-sm text-muted-foreground">
					{#each metrics.spoolsByStatus as c (c.label)}
						<li class="flex justify-between"><span>{c.label}</span><span class="font-medium text-foreground">{c.count}</span></li>
					{/each}
				</ul>
			</Card.Root>
			<Card.Root class="p-5">
				<h3 class="mb-3 text-sm font-semibold">Prints by outcome</h3>
				<ul class="space-y-1 text-sm text-muted-foreground">
					{#each metrics.printsByStatus as c (c.label)}
						<li class="flex justify-between"><span>{c.label}</span><span class="font-medium text-foreground">{c.count}</span></li>
					{/each}
				</ul>
			</Card.Root>
			<Card.Root class="p-5">
				<h3 class="mb-3 text-sm font-semibold">Filaments by brand (top 10)</h3>
				<ul class="space-y-1 text-sm text-muted-foreground">
					{#each metrics.filamentsByBrand.slice(0, 10) as c (c.label)}
						<li class="flex justify-between"><span>{c.label}</span><span class="font-medium text-foreground">{c.count}</span></li>
					{/each}
				</ul>
			</Card.Root>
			<Card.Root class="p-5">
				<h3 class="mb-3 text-sm font-semibold">Filaments by material (top 10)</h3>
				<ul class="space-y-1 text-sm text-muted-foreground">
					{#each metrics.filamentsByMaterial.slice(0, 10) as c (c.label)}
						<li class="flex justify-between"><span>{c.label}</span><span class="font-medium text-foreground">{c.count}</span></li>
					{/each}
				</ul>
			</Card.Root>
		</div>
	{/if}
</div>
