<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { listProfileVersions, selectProfileVersion, setProfileVersionName } from '$lib/api/client';
	import type { ProfileVersion } from '$lib/api/client';
	import { formHref } from '$lib/form-url';
	import { page } from '$app/state';

	// Versions panel (006/FR-042): a profile's versions are working variants — open any one, pick
	// which one lists and pickers use ("Use this version"), and name them ("Winter 2026").
	let { id, onSelected }: { id: number; onSelected?: (versionId: number) => void } = $props();

	let versions = $state<ProfileVersion[]>([]);
	let renaming = $state<{ id: number; name: string } | null>(null);

	async function load() {
		versions = await listProfileVersions(id);
	}

	async function use(versionId: number) {
		await selectProfileVersion(versionId);
		await load();
		onSelected?.(versionId);
	}

	async function saveName(e: Event) {
		e.preventDefault();
		if (!renaming) return;
		await setProfileVersionName(renaming.id, renaming.name);
		renaming = null;
		await load();
	}

	$effect(() => {
		load();
	});
</script>

<Card.Root>
	<Card.Header>
		<Card.Title>Versions</Card.Title>
	</Card.Header>
	<Card.Content>
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Version</Table.Head>
					<Table.Head>Name</Table.Head>
					<Table.Head>Nozzle</Table.Head>
					<Table.Head>Created</Table.Head>
					<Table.Head></Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each versions as v (v.id)}
					<Table.Row class={v.id === id ? 'bg-muted/50' : ''}>
						<Table.Cell>
							<span class="flex items-center gap-1.5">
								v{v.versionNumber}
								{#if v.selected}<Badge>Selected</Badge>{/if}
								{#if v.locked}<Badge variant="outline">Locked</Badge>{/if}
							</span>
						</Table.Cell>
						<Table.Cell>
							{#if renaming?.id === v.id}
								<form onsubmit={saveName} class="flex items-center gap-2">
									<!-- svelte-ignore a11y_autofocus -- opens only on an explicit Rename click -->
									<Input bind:value={renaming.name} placeholder="e.g. Winter 2026" class="h-8 w-40" autofocus />
									<Button type="submit" size="sm">Save</Button>
									<Button type="button" size="sm" variant="ghost" onclick={() => (renaming = null)}>Cancel</Button>
								</form>
							{:else}
								<span class={v.versionName ? '' : 'text-muted-foreground'}>{v.versionName ?? '—'}</span>
								<button type="button" class="ml-2 text-xs text-muted-foreground hover:underline" onclick={() => (renaming = { id: v.id, name: v.versionName ?? '' })}>
									Rename
								</button>
							{/if}
						</Table.Cell>
						<Table.Cell>{v.nozzleTempC} °C</Table.Cell>
						<Table.Cell>{new Date(v.createdAt).toLocaleDateString()}</Table.Cell>
						<Table.Cell class="text-right whitespace-nowrap">
							{#if v.id !== id}<a href={formHref(page.url, v.id)} class="text-sm hover:underline">Open</a>{/if}
							{#if !v.selected}
								<Button type="button" size="sm" variant="outline" class="ml-2" onclick={() => use(v.id)}>Use this version</Button>
							{/if}
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</Card.Content>
</Card.Root>
