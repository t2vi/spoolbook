<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import { getPrint, getProjectPlates } from '$lib/api/client';
	import type { FailureMode, Print, ProjectPlate } from '$lib/api/types';
	import { page } from '$app/state';

	const FAILURE_MODE_LABELS: Record<FailureMode, string> = {
		Stringing: 'Stringing',
		LayerAdhesion: 'Layer Adhesion',
		Warping: 'Warping',
		UnderExtrusion: 'Under-extrusion',
		OverExtrusion: 'Over-extrusion',
		LayerShift: 'Layer Shift',
		Clog: 'Clog',
		Other: 'Other'
	};

	let id = $derived(Number(page.params.id));
	let print = $state<Print | null>(null);
	let plates = $state<ProjectPlate[]>([]);

	let plate = $derived(plates.find((p) => p.platerId === print?.projectPlaterId) ?? null);

	function statusBadgeClass(status: Print['status']) {
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
		(async () => {
			print = await getPrint(id);
			plates = print.project ? await getProjectPlates(print.project.id) : [];
		})();
	});
</script>

<svelte:head>
	<title>Print details</title>
</svelte:head>

<div class="mx-auto max-w-2xl px-4 py-8">
	{#if print === null}
		<Card.Root>
			<Card.Header>
				<Skeleton class="h-7 w-40" />
			</Card.Header>
			<Card.Content class="space-y-4">
				<Skeleton class="h-48 w-full" />
				{#each Array(4) as _, i (i)}
					<Skeleton class="h-4 w-full" />
				{/each}
			</Card.Content>
		</Card.Root>
	{:else}
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-2xl">Print details</Card.Title>
				<Card.Action><Badge class={statusBadgeClass(print.status)}>{print.status}</Badge></Card.Action>
			</Card.Header>

			<Card.Content class="space-y-6">
				{#if print.project}
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Project</span>
						<span class="text-sm">{print.project.fileName}{plate ? ` — ${plate.platerName ?? `Plate ${plate.platerId}`}` : ''}</span>
						{#if plate?.thumbnailBytes}
							<div class="flex h-48 items-center justify-center overflow-hidden rounded-lg bg-slate-900">
								<img src="data:image/png;base64,{plate.thumbnailBytes}" class="h-full w-full object-contain" alt="Plate preview" />
							</div>
						{/if}
					</div>
					<Separator />
				{/if}

				<div class="grid grid-cols-2 gap-x-6 gap-y-4 text-sm">
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Spool</span>
						<span>{print.spool?.filament?.brand} {print.spool?.filament?.material} {print.spool?.filament?.variant ?? ''} — {print.spool?.filament?.color}{print.spool
								?.lotCode
								? ` (${print.spool.lotCode})`
								: ''}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Profile</span>
						<span>{print.profile?.name}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Printer</span>
						<span>{print.printer?.name}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Clean build plate</span>
						<span>{print.cleanBuildPlate ? 'Yes' : 'No'}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Started</span>
						<span>{new Date(print.startedAt).toLocaleString()}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Ended</span>
						<span>{print.endedAt ? new Date(print.endedAt).toLocaleString() : '—'}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">AMS humidity</span>
						<span>{print.amsHumidityPct !== null ? `${print.amsHumidityPct}%` : '—'}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Actual room temp</span>
						<span>{print.actualRoomTempC !== null ? `${print.actualRoomTempC}°C` : '—'}</span>
					</div>
					{#if print.ambientTempC !== null || print.ambientHumidityPct !== null}
						<div class="col-span-2 flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Ambient weather (auto-fetched)</span>
							<span
								>{print.ambientTempC !== null ? `${print.ambientTempC.toFixed(1)}°C` : '—'} / {print.ambientHumidityPct !== null
									? `${print.ambientHumidityPct.toFixed(0)}% humidity`
									: '—'}</span
							>
						</div>
					{/if}
				</div>

				{#if print.failureModes.length > 0}
					<Separator />
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Failure modes</span>
						<div class="flex flex-wrap gap-2">
							{#each print.failureModes as f (f.mode)}
								<Badge variant="outline">{FAILURE_MODE_LABELS[f.mode]}</Badge>
							{/each}
						</div>
					</div>
				{/if}

				{#if print.notes}
					<Separator />
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Notes</span>
						<p class="text-sm whitespace-pre-wrap">{print.notes}</p>
					</div>
				{/if}
			</Card.Content>

			<Card.Footer>
				<Button href="/prints" variant="ghost" size="sm">Back to prints</Button>
			</Card.Footer>
		</Card.Root>
	{/if}
</div>
