<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import CylinderIcon from '@lucide/svelte/icons/cylinder';
	import Disc3Icon from '@lucide/svelte/icons/disc-3';
	import PrinterIcon from '@lucide/svelte/icons/printer';
	import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
	import { getDashboard } from '$lib/api/client';
	import type { CategoryCount, DashboardSnapshot } from '$lib/api/types';
	import { formatDateTime } from '$lib/utils.js';

	let snapshot = $state<DashboardSnapshot | null>(null);

	$effect(() => {
		getDashboard().then((s) => (snapshot = s));
	});

	const sum = (counts: CategoryCount[]) => counts.reduce((total, c) => total + c.count, 0);

	let tiles = $derived(
		snapshot && [
			{ href: '/filaments', label: 'Filaments', value: snapshot.metrics.filamentCount, icon: CylinderIcon, tint: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' },
			{ href: '/spools', label: 'Spools', value: sum(snapshot.metrics.spoolsByStatus), icon: Disc3Icon, tint: 'bg-amber-500/10 text-amber-600 dark:text-amber-400' },
			{ href: '/profiles', label: 'Print Profiles', value: snapshot.profileCount, icon: SlidersHorizontalIcon, tint: 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400' },
			{ href: '/prints', label: 'Prints logged', value: sum(snapshot.metrics.printsByStatus), icon: PrinterIcon, tint: 'bg-rose-500/10 text-rose-600 dark:text-rose-400' }
		]
	);

	let breakdowns = $derived(
		snapshot && [
			{ title: 'Spools by status', counts: snapshot.metrics.spoolsByStatus },
			{ title: 'Prints by outcome', counts: snapshot.metrics.printsByStatus },
			{ title: 'Filaments by brand (top 10)', counts: snapshot.metrics.filamentsByBrand.slice(0, 10) },
			{ title: 'Filaments by material (top 10)', counts: snapshot.metrics.filamentsByMaterial.slice(0, 10) }
		]
	);
</script>

<svelte:head>
	<title>Spoolbook</title>
</svelte:head>

<div class="mx-auto w-full max-w-6xl px-4 pb-8 md:px-6">
	<div class="mb-6 flex flex-wrap items-baseline justify-between gap-2">
		<h1 class="text-2xl font-semibold">Dashboard</h1>
		{#if snapshot?.metrics.lastFilamentSyncAt}
			<p class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
				Catalog synced {formatDateTime(snapshot.metrics.lastFilamentSyncAt)}
			</p>
		{/if}
	</div>

	<div class="mb-6 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
		{#if tiles === null}
			{#each Array(4) as _, i (i)}
				<div class="flex items-center gap-4 rounded-xl border bg-card p-5 shadow-xs">
					<Skeleton class="size-12 rounded-full" />
					<div class="space-y-2"><Skeleton class="h-4 w-20" /><Skeleton class="h-7 w-12" /></div>
				</div>
			{/each}
		{:else}
			{#each tiles as t (t.href)}
				<a href={t.href} class="flex items-center gap-4 rounded-xl border bg-card p-5 shadow-xs transition hover:shadow-md">
					<span class="flex size-12 shrink-0 items-center justify-center rounded-full {t.tint}"><t.icon class="size-5" /></span>
					<span class="flex flex-col">
						<span class="text-sm text-muted-foreground">{t.label}</span>
						<span class="text-3xl font-bold tracking-tight">{t.value.toLocaleString()}</span>
					</span>
				</a>
			{/each}
		{/if}
	</div>

	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
		{#if breakdowns === null}
			{#each Array(4) as _, i (i)}
				<div class="rounded-xl border bg-card shadow-xs">
					<div class="border-b px-5 py-4"><Skeleton class="h-5 w-40" /></div>
					<div class="space-y-3 p-5">
						{#each Array(3) as _, j (j)}<Skeleton class="h-4 w-full" />{/each}
					</div>
				</div>
			{/each}
		{:else}
			{#each breakdowns as b (b.title)}
				{@const total = sum(b.counts)}
				<section class="rounded-xl border bg-card shadow-xs">
					<h3 class="border-b px-5 py-4 font-semibold">{b.title}</h3>
					<ul class="space-y-3 p-5 text-sm">
						{#each b.counts as c, i (c.label)}
							<li>
								<div class="flex justify-between"><span class="text-muted-foreground">{c.label}</span><span class="font-semibold">{c.count}</span></div>
								<div class="mt-1.5 h-1.5 rounded-full bg-muted">
									<div class="h-full rounded-full" style="width: {total ? (c.count / total) * 100 : 0}%; background: var(--chart-{(i % 5) + 1})"></div>
								</div>
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		{/if}
	</div>
</div>
