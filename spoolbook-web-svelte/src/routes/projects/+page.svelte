<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import { deleteProject, listProjects, me, renameProject } from '$lib/api/client';
	import type { Project } from '$lib/api/types';

	let projects = $state<Project[]>([]);
	let authenticated = $state(false);
	let errorMessage = $state<string | null>(null);
	let selected = $state<Set<number>>(new Set());
	let renamingId = $state<number | null>(null);
	let renameValue = $state('');

	// null = closed. 'bulk' or a project id = confirm prompt open. blockedNames set = the delete
	// guard fired (has_prints) -- dialog switches to a single-button acknowledgement instead of
	// silently falling back to the page's plain error line.
	let deleteTarget = $state<number | 'bulk' | null>(null);
	let blockedNames = $state<string[] | null>(null);

	async function load() {
		projects = await listProjects();
		selected = new Set([...selected].filter((id) => projects.some((p) => p.id === id)));
	}

	function toggleSelected(id: number, checked: boolean) {
		const next = new Set(selected);
		if (checked) next.add(id);
		else next.delete(id);
		selected = next;
	}

	function toggleSelectAll(checked: boolean) {
		selected = checked ? new Set(projects.map((p) => p.id)) : new Set();
	}

	function startRename(project: Project) {
		renamingId = project.id;
		renameValue = project.fileName;
	}

	async function confirmRename() {
		if (renamingId === null) return;
		if (!renameValue.trim()) {
			errorMessage = 'Name is required.';
			return;
		}
		const result = await renameProject(renamingId, renameValue.trim());
		if (!result.ok) {
			errorMessage = result.error ?? 'Rename failed.';
			return;
		}
		errorMessage = null;
		renamingId = null;
		await load();
	}

	function askDelete(id: number) {
		blockedNames = null;
		deleteTarget = id;
	}

	function askDeleteSelected() {
		blockedNames = null;
		deleteTarget = 'bulk';
	}

	function closeDeleteDialog() {
		deleteTarget = null;
		blockedNames = null;
	}

	async function confirmDelete() {
		if (deleteTarget === null) return;
		errorMessage = null;

		if (deleteTarget !== 'bulk') {
			const project = projects.find((p) => p.id === deleteTarget);
			const result = await deleteProject(deleteTarget);
			if (!result.ok) {
				blockedNames = [project?.fileName ?? `#${deleteTarget}`];
				return;
			}
			deleteTarget = null;
			await load();
			return;
		}

		const blocked: string[] = [];
		for (const id of selected) {
			const result = await deleteProject(id);
			if (!result.ok) blocked.push(projects.find((p) => p.id === id)?.fileName ?? `#${id}`);
		}
		selected = new Set();
		await load();
		if (blocked.length > 0) {
			blockedNames = blocked;
		} else {
			deleteTarget = null;
		}
	}

	$effect(() => {
		load();
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Projects</title>
</svelte:head>

<div class="mx-auto max-w-4xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Projects</h1>
	<p class="mb-4 text-sm text-muted-foreground">
		Every uploaded or re-sliced <code>.3mf</code> project. Re-slicing supersedes the previous version instead of adding a duplicate here.
	</p>

	{#if authenticated && selected.size > 0}
		<div class="mb-4">
			<Button variant="destructive" onclick={askDeleteSelected}>Delete {selected.size} selected</Button>
		</div>
	{/if}

	{#if projects.length === 0}
		<p class="text-muted-foreground">No projects yet — upload or send a print to add one.</p>
	{:else}
		<div class="rounded-lg border bg-card shadow-sm">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						{#if authenticated}
							<Table.Head class="w-10">
								<Checkbox checked={projects.length > 0 && selected.size === projects.length} onCheckedChange={(v: boolean) => toggleSelectAll(v)} />
							</Table.Head>
						{/if}
						<Table.Head>Name</Table.Head>
						<Table.Head></Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each projects as p (p.id)}
						<Table.Row>
							{#if authenticated}
								<Table.Cell>
									<Checkbox checked={selected.has(p.id)} onCheckedChange={(v: boolean) => toggleSelected(p.id, v)} />
								</Table.Cell>
							{/if}
							<Table.Cell>
								{#if renamingId === p.id}
									<div class="flex items-center gap-2">
										<Input bind:value={renameValue} class="h-8 max-w-xs" onkeydown={(e) => e.key === 'Enter' && confirmRename()} />
										<Button size="sm" onclick={confirmRename}>Save</Button>
										<button type="button" class="text-sm text-muted-foreground hover:text-foreground" onclick={() => (renamingId = null)}>Cancel</button>
									</div>
								{:else}
									<a href="/projects/{p.id}" class="hover:underline">{p.fileName}</a>
								{/if}
							</Table.Cell>
							<Table.Cell>
								{#if renamingId !== p.id}
									<a href="/projects/{p.id}" class="hover:underline">View</a>
									{#if authenticated}
										<button type="button" onclick={() => startRename(p)} class="ml-3 hover:underline">Rename</button>
										<button type="button" onclick={() => askDelete(p.id)} class="ml-3 text-destructive hover:underline">Delete</button>
									{/if}
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}

	{#if !authenticated}<p class="mt-3 text-sm text-muted-foreground"><a href="/login" class="underline">Sign in</a> to rename or delete.</p>{/if}
	{#if errorMessage}<p class="mt-3 text-sm text-destructive">{errorMessage}</p>{/if}
</div>

<AlertDialog.Root open={deleteTarget !== null} onOpenChange={(open) => !open && closeDeleteDialog()}>
	<AlertDialog.Content class="sm:max-w-md">
		{#if blockedNames}
			<AlertDialog.Header>
				<AlertDialog.Title>Can't delete</AlertDialog.Title>
				<AlertDialog.Description>
					{#if blockedNames.length === 1}
						"{blockedNames[0]}" is referenced by a Print — delete that Print first, or keep this project.
					{:else}
						These are referenced by a Print and were skipped: {blockedNames.join(', ')}. Anything else selected was deleted.
					{/if}
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Action onclick={closeDeleteDialog}>OK</AlertDialog.Action>
			</AlertDialog.Footer>
		{:else}
			<AlertDialog.Header>
				<AlertDialog.Title>Delete {deleteTarget === 'bulk' ? `${selected.size} projects` : 'project'}?</AlertDialog.Title>
				<AlertDialog.Description>
					{#if deleteTarget === 'bulk'}
						This deletes {selected.size} selected project{selected.size === 1 ? '' : 's'}. This can't be undone.
					{:else}
						This deletes "{projects.find((p) => p.id === deleteTarget)?.fileName}". This can't be undone.
					{/if}
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
				<Button variant="destructive" onclick={confirmDelete}>Delete</Button>
			</AlertDialog.Footer>
		{/if}
	</AlertDialog.Content>
</AlertDialog.Root>
