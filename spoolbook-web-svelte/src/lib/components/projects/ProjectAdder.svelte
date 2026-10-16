<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { getProject, importProjectFromUrl, linkProjectVersion, projectVersionCandidate, uploadProject } from '$lib/api/client';
	import type { Project, ProjectResult } from '$lib/api/types';

	// Add a Project by upload or from a pasted direct .3mf link (005/FR-042). A newly created one
	// that looks like the next version of an existing Project asks to link them (005/FR-043);
	// re-adding identical bytes (created: false) never asks. `onAdded` gets the project, and again
	// after a link (its version number changes).
	let { onAdded }: { onAdded: (project: Project) => void } = $props();

	let url = $state('');
	let busy = $state<'upload' | 'fetch' | null>(null);
	let error = $state<string | null>(null);
	let prompt = $state<{ project: Project; candidate: Project } | null>(null);

	async function add(kind: 'upload' | 'fetch', run: () => Promise<ProjectResult>) {
		busy = kind;
		error = null;
		prompt = null;
		try {
			const result = await run();
			if (!result.ok || !result.project) {
				error = result.error ?? 'Import failed.';
				return;
			}
			onAdded(result.project);
			if (kind === 'fetch') url = '';
			if (!result.created) return;
			const candidate = await projectVersionCandidate(result.project);
			if (candidate) prompt = { project: result.project, candidate };
		} catch (err) {
			error = err instanceof Error ? err.message : 'Import failed.';
		} finally {
			busy = null;
		}
	}

	function onFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		add('upload', () => uploadProject(file)).then(() => (input.value = ''));
	}

	function onFetch(e: Event) {
		e.preventDefault();
		if (!url.trim()) return;
		add('fetch', () => importProjectFromUrl(url.trim()));
	}

	async function link() {
		if (!prompt) return;
		const { project, candidate } = prompt;
		prompt = null;
		try {
			await linkProjectVersion(project.id, candidate.id);
			onAdded(await getProject(project.id));
		} catch (err) {
			error = `Link failed: ${err instanceof Error ? err.message : 'unknown error'}`;
		}
	}
</script>

<div class="flex flex-col gap-2">
	<div class="flex items-center gap-2">
		<input
			type="file"
			accept=".3mf"
			onchange={onFile}
			disabled={busy !== null}
			class="text-sm file:mr-3 file:cursor-pointer file:rounded-md file:border file:bg-background file:px-3 file:py-1.5 file:text-sm file:font-medium hover:file:bg-accent"
		/>
		{#if busy === 'upload'}<span class="text-xs text-muted-foreground">Uploading…</span>{/if}
	</div>
	<form onsubmit={onFetch} class="flex items-center gap-2">
		<Input bind:value={url} type="url" placeholder="or paste a direct .3mf URL" class="h-8 text-sm" disabled={busy !== null} />
		<Button type="submit" size="sm" variant="outline" disabled={busy !== null || !url.trim()}>{busy === 'fetch' ? 'Fetching…' : 'Fetch'}</Button>
	</form>
	{#if error}<p class="text-xs text-destructive">{error}</p>{/if}
	{#if prompt}
		<div class="flex flex-wrap items-center gap-2 rounded-md border bg-muted/50 px-3 py-2 text-sm">
			<span>Looks like a new version of <strong>{prompt.candidate.fileName}</strong> — link it?</span>
			<Button size="sm" onclick={link}>Link</Button>
			<Button size="sm" variant="ghost" onclick={() => (prompt = null)}>Not now</Button>
		</div>
	{/if}
</div>
