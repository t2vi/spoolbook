<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as InputGroup from '$lib/components/ui/input-group/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import Picker from '$lib/components/picker.svelte';
	import FilamentPicker from '$lib/components/filament-picker.svelte';
	import SearchablePicker from '$lib/components/searchable-picker.svelte';
	import {
		createProfile,
		getProfileFieldSpec,
		importProfilePreset,
		listAllFilaments,
		listSystemPresets,
		resolveSystemPreset,
		updateProfile
	} from '$lib/api/client';
	import type { ProfileInput } from '$lib/api/client';
	import type { Filament, ImportResult, ProfileFieldTab, ProfileSource, SlicerType } from '$lib/api/types';
	import { goto } from '$app/navigation';

	let { id }: { id: number | null } = $props();

	// Browsers won't let a page set the file dialog's starting folder (that'd let a site probe
	// filesystem structure), so this is just a hint for where to navigate the first time --
	// the browser itself remembers the last folder used for this input on subsequent uploads.
	// Real per-OS paths lifted from the retired .NET BambuPaths.cs (only macOS was ever verified
	// against a live install there; Windows/Linux carry the same "reasonable guess" caveat).
	const bambuPresetsDirHint = (() => {
		if (typeof navigator === 'undefined') return null;
		const platform = navigator.userAgent;
		if (/Mac/i.test(platform)) return '~/Library/Application Support/BambuStudio/user/<your account>/filament/';
		if (/Win/i.test(platform)) return '%APPDATA%\\BambuStudio\\user\\<your account>\\filament\\';
		if (/Linux/i.test(platform)) return '~/.config/BambuStudio/user/<your account>/filament/';
		return null;
	})();

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

	let systemPresetOptions = $state<{ value: string; label: string }[]>([]);
	let selectedSystemPreset = $state('');

	$effect(() => {
		(async () => {
			if (id === null) {
				filaments = await listAllFilaments();
				// slicer-service (source of the Bambu-defaults list) isn't always running --
				// e.g. local dev without it started -- and that's fine, the rest of the form
				// (including .3mf/preset import) works without it. Only that one dropdown
				// degrades to empty, nothing else should be affected by it being down.
				try {
					const presets = await listSystemPresets();
					systemPresetOptions = presets.names.map((n) => ({ value: n, label: n }));
				} catch {
					systemPresetOptions = [];
				}
			}

			const spec = await getProfileFieldSpec(id);
			name = spec.name;
			tabs = spec.tabs;
			activeTab = tabs[0]?.title ?? '';
		})();
	});

	// ShowRow/BoolValue are computed live off the current value in the Blazor original
	// (ProfileFieldEntry is a mutable ObservableObject) — recomputed inline here for the same
	// reason: the server's snapshot goes stale the instant a user edits a field.
	function showRow(field: { hideWhenBlank: boolean; value: string }) {
		return !field.hideWhenBlank || field.value.trim() !== '';
	}

	async function applyImportResult(result: ImportResult, importedFrom: string) {
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

		if (!name.trim()) name = result.suggestedName ?? importedFrom;

		rawSettingsJson = result.rawSettingsJson;
		source = 'SlicerImport';
		sourceSlicer = 'BambuStudio';
		importMessage = `Imported ${Object.keys(result.fields).length} settings from ${importedFrom}.`;
	}

	async function onFileSelected(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		try {
			await applyImportResult(await importProfilePreset(file), file.name);
		} catch (err) {
			importMessage = `Import failed: ${err instanceof Error ? err.message : 'unknown error'}`;
		}
	}

	async function onSystemPresetSelected(newValue: string) {
		if (!newValue) return;
		try {
			await applyImportResult(await resolveSystemPreset(newValue), newValue);
		} catch (err) {
			importMessage = `Import failed: ${err instanceof Error ? err.message : 'unknown error'}`;
		}
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

<div class="max-w-3xl space-y-6">
	{#if id === null}
		<Card.Root>
			<Card.Header>
				<Card.Title>Start from</Card.Title>
				<Card.Description>Pick a filament, then optionally seed the fields below from a Bambu Studio default or an imported preset.</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-4">
				<div class="flex flex-col gap-1">
					<Label for="filament-select">Filament</Label>
					<FilamentPicker id="filament-select" bind:value={selectedFilamentId} {filaments} />
				</div>

				<div class="flex flex-col gap-1">
					<Label for="system-preset">Start from a Bambu Studio default (optional)</Label>
					<SearchablePicker
						id="system-preset"
						bind:value={selectedSystemPreset}
						options={systemPresetOptions}
						placeholder="-- pick a Bambu default --"
						searchPlaceholder="Search Bambu presets…"
						onValueChange={onSystemPresetSelected}
					/>
				</div>

				<div class="flex flex-col gap-1">
					<Label for="import-preset">Or import a sliced .3mf / Bambu Studio preset (.json) (optional)</Label>
					<input
						id="import-preset"
						type="file"
						accept=".3mf,.json"
						onchange={onFileSelected}
						class="text-sm file:mr-3 file:cursor-pointer file:rounded-md file:border file:bg-background file:px-3 file:py-1.5 file:text-sm file:font-medium hover:file:bg-accent"
					/>
					{#if bambuPresetsDirHint}
						<p class="text-xs text-muted-foreground">
							Your saved presets live in <code class="rounded bg-muted px-1 py-0.5">{bambuPresetsDirHint}</code> — the browser remembers this
							folder after your first visit here.
						</p>
					{/if}
					{#if importMessage}<p class="text-sm text-muted-foreground">{importMessage}</p>{/if}
				</div>
			</Card.Content>
		</Card.Root>
	{/if}

	<Card.Root>
		<Card.Content class="pt-6">
			<div class="flex flex-col gap-1">
				<Label for="name">Name</Label>
				<Input id="name" bind:value={name} />
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Content class="pt-6">
			<Tabs.Root bind:value={activeTab}>
				<Tabs.List class="mb-4">
					{#each tabs as tab (tab.title)}
						<Tabs.Trigger value={tab.title}>{tab.title}</Tabs.Trigger>
					{/each}
				</Tabs.List>

				{#each tabs as tab (tab.title)}
					<Tabs.Content value={tab.title} class="min-h-[420px] space-y-6">
						{#each tab.sections as section, i (section.title)}
							{#if i > 0}<Separator />{/if}
							<div>
								<h3 class="mb-3 text-sm font-semibold tracking-wide text-muted-foreground uppercase">{section.title}</h3>
								<div class="grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2">
									{#each section.fields.filter(showRow) as field (field.name)}
										{#if field.isBool}
											<div class="flex items-center gap-2">
												<Checkbox
													id="field-{field.name}"
													checked={field.value === 'true'}
													onCheckedChange={(v: boolean) => (field.value = v ? 'true' : 'false')}
												/>
												<Label for="field-{field.name}">{field.label}{field.unit ? ` (${field.unit})` : ''}</Label>
											</div>
										{:else}
											<div class="flex flex-col gap-1">
												<Label for="field-{field.name}">
													{field.label}{field.unit && (field.isEnum || field.isTextArea) ? ` (${field.unit})` : ''}
												</Label>
												{#if field.isEnum}
													<Picker
														id="field-{field.name}"
														bind:value={field.value}
														options={[{ value: '', label: '--' }, ...(field.options ?? []).map((opt) => ({ value: opt, label: opt }))]}
													/>
												{:else if field.isTextArea}
													<Textarea id="field-{field.name}" bind:value={field.value} rows={4} class="font-mono text-xs" />
												{:else if field.unit}
													<InputGroup.Root>
														<InputGroup.Input id="field-{field.name}" bind:value={field.value} />
														<InputGroup.Addon align="inline-end">
															<InputGroup.Text>{field.unit}</InputGroup.Text>
														</InputGroup.Addon>
													</InputGroup.Root>
												{:else}
													<Input id="field-{field.name}" bind:value={field.value} />
												{/if}
											</div>
										{/if}
									{/each}
								</div>
							</div>
						{/each}
					</Tabs.Content>
				{/each}
			</Tabs.Root>
		</Card.Content>
	</Card.Root>

	{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

	<form onsubmit={save} class="flex items-center gap-3">
		<Button type="submit">Save</Button>
		<a href="/profiles" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
	</form>
</div>
