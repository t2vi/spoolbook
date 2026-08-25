<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import { getProject, getProjectPlates, listPrintsForProject } from '$lib/api/client';
	import type { Print, PrintStatus, Project, ProjectPlate } from '$lib/api/types';
	import { page } from '$app/state';
	import { formatDateTime } from '$lib/utils.js';

	let id = $derived(Number(page.params.id));
	let project = $state<Project | null>(null);
	let plates = $state<ProjectPlate[]>([]);
	let prints = $state<Print[]>([]);

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

	function formatBytes(bytes: number) {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	$effect(() => {
		(async () => {
			project = await getProject(id);
			[plates, prints] = await Promise.all([getProjectPlates(id), listPrintsForProject(id)]);
		})();
	});
</script>

<svelte:head>
	<title>Project details</title>
</svelte:head>

<div class="mx-auto max-w-2xl px-4 py-8">
	{#if project === null}
		<Card.Root>
			<Card.Header>
				<Skeleton class="h-7 w-56" />
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
				<Card.Title class="text-2xl">{project.fileName}</Card.Title>
				<Card.Action>
					<Badge variant={project.isCurrentVersion ? 'default' : 'outline'}>
						v{project.versionNumber}{project.isCurrentVersion ? '' : ' (superseded)'}
					</Badge>
				</Card.Action>
			</Card.Header>

			<Card.Content class="space-y-6">
				{#if plates.length > 0}
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Plates</span>
						<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
							{#each plates as plate (plate.platerId)}
								<div class="flex flex-col gap-1">
									<div class="flex h-28 items-center justify-center overflow-hidden rounded-lg bg-slate-900">
										{#if plate.thumbnailBytes}
											<img src="data:image/png;base64,{plate.thumbnailBytes}" class="h-full w-full object-contain" alt="Plate preview" />
										{:else}
											<span class="text-xs text-slate-400">No preview</span>
										{/if}
									</div>
									<span class="text-xs text-muted-foreground">{plate.platerName ?? `Plate ${plate.platerId}`}</span>
								</div>
							{/each}
						</div>
					</div>
					<Separator />
				{/if}

				<div class="grid grid-cols-2 gap-x-6 gap-y-4 text-sm">
					<div class="col-span-2 flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">File path</span>
						<span class="font-mono text-xs break-all">{project.filePath}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Last updated</span>
						<span>{formatDateTime(project.lastKnownWriteTimeUtc)}</span>
					</div>
					<div class="flex flex-col gap-1">
						<span class="font-medium text-muted-foreground">Size</span>
						<span>{formatBytes(project.lastKnownFileSizeBytes)}</span>
					</div>
					{#if project.meshHash}
						<div class="col-span-2 flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Mesh hash</span>
							<span class="font-mono text-xs break-all">{project.meshHash}</span>
						</div>
					{/if}
					{#if project.previousVersionProjectId}
						<div class="col-span-2 flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Previous version</span>
							<a href="/projects/{project.previousVersionProjectId}" class="text-sm hover:underline">View v{project.versionNumber - 1}</a>
						</div>
					{/if}
				</div>

				<Separator />

				<div class="flex flex-col gap-2">
					<span class="text-sm font-medium text-muted-foreground">Prints from this project</span>
					{#if prints.length === 0}
						<p class="text-sm text-muted-foreground">No prints logged yet.</p>
					{:else}
						<ul class="divide-y rounded-lg border">
							{#each prints as p (p.id)}
								<li class="flex items-center justify-between px-3 py-2 text-sm">
									<span>{formatDateTime(p.startedAt)}</span>
									<div class="flex items-center gap-3">
										<Badge class={statusBadgeClass(p.status)}>{p.status}</Badge>
										<a href="/prints/{p.id}" class="text-muted-foreground hover:text-foreground hover:underline">View</a>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</Card.Content>

			<Card.Footer>
				<Button href="/projects" variant="ghost" size="sm">Back to projects</Button>
			</Card.Footer>
		</Card.Root>
	{/if}
</div>
