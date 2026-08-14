<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import Picker from '$lib/components/picker.svelte';
	import {
		createProfile,
		getProfileFieldSpec,
		importProfileFrom3mf,
		listAllFilaments,
		updateProfile
	} from '$lib/api/client';
	import type { ProfileInput } from '$lib/api/client';
	import type { Filament, ProfileFieldTab, ProfileSource, SlicerType } from '$lib/api/types';
	import { goto } from '$app/navigation';

	let { id }: { id: number | null } = $props();

	let filaments = $state<Filament[]>([]);
	let selectedFilamentId = $state(0);
	let name = $state('');
	let tabs = $state<ProfileFieldTab[]>([]);
	let activeTab = $state('');
	let rawSettingsJson = $state<string | null>(null);
	let source = $state<ProfileSource>('Manual');
	let sourceSlicer = $state<SlicerType | null>(null);
	let importMessage = $state<string | null>(null);
	let errorMessage = $state<string | null>(null);

	$effect(() => {
		(async () => {
			if (id === null) filaments = await listAllFilaments();

			const spec = await getProfileFieldSpec(id);
			name = spec.name;
			tabs = spec.tabs;
			activeTab = tabs[0]?.title ?? '';
		})();
	});

	function tabButtonClass(title: string) {
		return title === activeTab
			? 'rounded-t-md border border-b-0 bg-card px-3 py-1.5 text-sm font-medium text-foreground'
			: 'rounded-t-md border border-b-0 border-transparent px-3 py-1.5 text-sm font-medium text-muted-foreground hover:text-foreground';
	}

	// ShowRow/BoolValue are computed live off the current value in the Blazor original
	// (ProfileFieldEntry is a mutable ObservableObject) — recomputed inline here for the same
	// reason: the server's snapshot goes stale the instant a user edits a field.
	function showRow(field: { hideWhenBlank: boolean; value: string }) {
		return !field.hideWhenBlank || field.value.trim() !== '';
	}

	async function onThreeMfSelected(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;

		const result = await importProfileFrom3mf(file);
		if (!result.ok || !result.fields) {
			importMessage = `Import failed: ${result.error}`;
			return;
		}

		const spec = await getProfileFieldSpec(null);
		for (const tab of spec.tabs) {
			for (const section of tab.sections) {
				for (const field of section.fields) {
					if (field.name in result.fields) field.value = result.fields[field.name];
				}
			}
		}
		tabs = spec.tabs;
		activeTab = tabs[0]?.title ?? '';

		if (!name.trim()) name = file.name.replace(/\.3mf$/i, '');

		rawSettingsJson = result.rawSettingsJson;
		source = 'SlicerImport';
		sourceSlicer = 'BambuStudio';
		importMessage = `Imported ${Object.keys(result.fields).length} settings from ${file.name}.`;
	}

	async function save(e: Event) {
		e.preventDefault();
		const fields: Record<string, string> = {};
		for (const tab of tabs) for (const section of tab.sections) for (const field of section.fields) fields[field.name] = field.value;

		const input: ProfileInput = { name, fields, source, sourceSlicer, rawSettingsJson, spoolId: null };

		const result = id === null
			? selectedFilamentId === 0
				? { ok: false, errors: { Filament: 'Pick a filament.' } }
				: await createProfile(selectedFilamentId, input)
			: await updateProfile(id, input);

		if (!result.ok) {
			errorMessage = Object.values(result.errors ?? {}).join('; ') || 'Save failed.';
			return;
		}

		goto('/profiles');
	}
</script>

<h1 class="mb-6 text-2xl font-semibold">{id === null ? 'Add profile' : 'Edit profile'}</h1>

<div class="max-w-3xl space-y-4">
	{#if id === null}
		<div class="flex flex-col gap-1">
			<Label for="filament-select">Filament</Label>
			<Picker
				id="filament-select"
				bind:value={selectedFilamentId}
				options={[
					{ value: 0, label: '-- pick a filament --' },
					...filaments.map((f) => ({ value: f.id, label: `${f.brand} ${f.material} ${f.variant ?? ''} — ${f.color}` }))
				]}
			/>
		</div>

		<div class="flex flex-col gap-1">
			<Label for="import-3mf">Import from a sliced .3mf (optional)</Label>
			<input id="import-3mf" type="file" accept=".3mf" onchange={onThreeMfSelected} class="text-sm" />
			{#if importMessage}<p class="text-sm text-muted-foreground">{importMessage}</p>{/if}
		</div>
	{/if}

	<div class="flex flex-col gap-1">
		<Label for="name">Name</Label>
		<Input id="name" bind:value={name} />
	</div>

	<div class="flex flex-wrap gap-1 border-b">
		{#each tabs as tab (tab.title)}
			<button type="button" onclick={() => (activeTab = tab.title)} class={tabButtonClass(tab.title)}>{tab.title}</button>
		{/each}
	</div>

	{#each tabs.filter((t) => t.title === activeTab) as tab (tab.title)}
		{#each tab.sections as section (section.title)}
			<div class="pt-2">
				<h3 class="mb-2 text-sm font-semibold tracking-wide text-muted-foreground uppercase">{section.title}</h3>
				<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
					{#each section.fields.filter(showRow) as field (field.name)}
						<div class="flex flex-col gap-1">
							<Label for="field-{field.name}">{field.label}{field.unit ? ` (${field.unit})` : ''}</Label>
							{#if field.isBool}
								<input
									id="field-{field.name}"
									type="checkbox"
									checked={field.value === 'true'}
									onchange={(e) => (field.value = e.currentTarget.checked ? 'true' : 'false')}
									class="h-4 w-4 self-start"
								/>
							{:else if field.isEnum}
								<Picker
									id="field-{field.name}"
									bind:value={field.value}
									options={[{ value: '', label: '--' }, ...(field.options ?? []).map((opt) => ({ value: opt, label: opt }))]}
								/>
							{:else if field.isTextArea}
								<Textarea id="field-{field.name}" bind:value={field.value} rows={4} class="font-mono text-xs" />
							{:else}
								<Input id="field-{field.name}" bind:value={field.value} />
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/each}
	{/each}

	{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

	<form onsubmit={save} class="flex items-center gap-3 pt-2">
		<Button type="submit">Save</Button>
		<a href="/profiles" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
	</form>
</div>
