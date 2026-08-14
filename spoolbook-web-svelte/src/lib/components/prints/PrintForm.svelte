<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import Picker from '$lib/components/picker.svelte';
	import {
		attachJobToPrint,
		createPrint,
		findJobMatch,
		findVersionCandidate,
		getPrint,
		getProjectPlates,
		importProjectFromUrl,
		linkProjectVersion,
		listPrinters,
		listProjects,
		listProfilesForFilament,
		listSpools,
		updatePrint,
		uploadProject
	} from '$lib/api/client';
	import type { PrintInput } from '$lib/api/client';
	import type { FailureMode, Printer, PrinterJob, PrintProfile, PrintStatus, Project, ProjectPlate, Spool } from '$lib/api/types';
	import { goto } from '$app/navigation';

	let { id }: { id: number | null } = $props();

	const ALL_STATUSES: PrintStatus[] = ['Success', 'Failed', 'Partial', 'InProgress'];
	const ALL_FAILURE_MODES: FailureMode[] = ['Stringing', 'LayerAdhesion', 'Warping', 'UnderExtrusion', 'OverExtrusion', 'LayerShift', 'Clog', 'Other'];
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

	let spools = $state<Spool[]>([]);
	let profiles = $state<PrintProfile[]>([]);
	let printers = $state<Printer[]>([]);
	let projects = $state<Project[]>([]);
	let plates = $state<ProjectPlate[]>([]);
	let selectedSpoolId = $state(0);
	let selectedProfileId = $state(0);
	let selectedPrinterId = $state(0);
	let selectedProjectId = $state(0);
	let selectedPlaterId = $state('');
	let uploading = $state(false);
	let uploadError = $state<string | null>(null);
	let versionCandidate = $state<Project | null>(null);
	let importUrl = $state('');
	let matchedJob = $state<PrinterJob | null>(null);
	let matchDismissed = $state(false);
	let startedAt = $state('');
	let endedAt = $state('');
	let status = $state<PrintStatus>('Success');
	let selectedFailureModes = $state<Set<FailureMode>>(new Set());
	let amsHumidityPct = $state('');
	let actualRoomTempC = $state('');
	let cleanBuildPlate = $state(false);
	let notes = $state('');
	let errorMessage = $state<string | null>(null);

	let selectedPlate = $derived(plates.find((p) => p.platerId === selectedPlaterId) ?? null);

	$effect(() => {
		(async () => {
			spools = await listSpools();
			printers = await listPrinters();
			projects = await listProjects();

			if (id === null) return;

			const existing = await getPrint(id);
			selectedSpoolId = existing.spoolId;
			selectedPrinterId = existing.printerId;
			startedAt = existing.startedAt.slice(0, 16);
			endedAt = existing.endedAt?.slice(0, 16) ?? '';
			status = existing.status;
			selectedFailureModes = new Set(existing.failureModes.map((f) => f.mode));
			amsHumidityPct = existing.amsHumidityPct?.toString() ?? '';
			actualRoomTempC = existing.actualRoomTempC?.toString() ?? '';
			cleanBuildPlate = existing.cleanBuildPlate ?? false;
			notes = existing.notes ?? '';

			if (existing.spool) {
				profiles = await listProfilesForFilament(existing.spool.filamentId);
				if (!profiles.some((p) => p.id === existing.profileId) && existing.profile) profiles = [...profiles, existing.profile];
			}
			selectedProfileId = existing.profileId;

			if (existing.project) {
				selectedProjectId = existing.project.id;
				selectedPlaterId = existing.projectPlaterId ?? '';
				plates = await getProjectPlates(existing.project.id);
			}
		})();
	});

	async function onSpoolChanged() {
		selectedProfileId = 0;
		const spool = spools.find((s) => s.id === selectedSpoolId);
		profiles = spool ? await listProfilesForFilament(spool.filamentId) : [];
	}

	async function onProjectChanged() {
		const project = projects.find((p) => p.id === selectedProjectId);
		plates = project ? await getProjectPlates(project.id) : [];
		selectedPlaterId = plates[0]?.platerId ?? '';
	}

	async function applyUploadResult(result: { ok: boolean; error?: string | null; project?: Project | null; created?: boolean }) {
		if (!result.ok || !result.project) {
			uploadError = `Import failed: ${result.error}`;
			return;
		}
		const project = result.project;
		if (!projects.some((p) => p.id === project.id)) projects = [...projects, project];

		selectedProjectId = project.id;
		plates = await getProjectPlates(project.id);
		selectedPlaterId = plates[0]?.platerId ?? '';

		// Only worth suggesting a version link for a genuinely new upload — re-uploading an
		// identical file just dedupes to the existing row, nothing to link.
		if (result.created) versionCandidate = await findVersionCandidate(project.meshHash, project.fileName, project.id);
	}

	async function onThreeMfSelected(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;

		uploading = true;
		uploadError = null;
		versionCandidate = null;
		try {
			await applyUploadResult(await uploadProject(file));
		} finally {
			uploading = false;
		}
	}

	async function fetchFromUrl() {
		if (!importUrl.trim()) return;
		uploading = true;
		uploadError = null;
		versionCandidate = null;
		try {
			const result = await importProjectFromUrl(importUrl);
			await applyUploadResult(result);
			if (result.ok) importUrl = '';
		} finally {
			uploading = false;
		}
	}

	async function linkVersion() {
		if (!versionCandidate) return;
		await linkProjectVersion(selectedProjectId, versionCandidate.id);
		versionCandidate = null;
	}

	// Only offered when logging a NEW print — an existing Print's Job (if any) was already
	// decided at creation time, matching the Blazor form's Id.HasValue guard.
	async function refreshJobMatch() {
		if (id !== null || selectedPrinterId === 0 || !startedAt) {
			matchedJob = null;
			return;
		}
		matchDismissed = false;
		matchedJob = await findJobMatch(selectedPrinterId, startedAt);
	}

	function toggleFailureMode(mode: FailureMode, checked: boolean) {
		const next = new Set(selectedFailureModes);
		if (checked) next.add(mode);
		else next.delete(mode);
		selectedFailureModes = next;
	}

	async function save(e: Event) {
		e.preventDefault();
		if (selectedSpoolId === 0) { errorMessage = 'Pick a spool.'; return; }
		if (selectedProfileId === 0) { errorMessage = 'Pick a profile.'; return; }
		if (selectedPrinterId === 0) { errorMessage = 'Pick a printer.'; return; }
		if (!startedAt || !endedAt) { errorMessage = 'Enter both start and end date/time.'; return; }

		const input: PrintInput = {
			startedAt,
			endedAt,
			status,
			notes: notes.trim() || null,
			amsHumidityPct: amsHumidityPct.trim() ? Number(amsHumidityPct) : null,
			actualRoomTempC: actualRoomTempC.trim() ? Number(actualRoomTempC) : null,
			cleanBuildPlate,
			projectId: selectedProjectId === 0 ? null : selectedProjectId,
			projectPlaterId: selectedProjectId === 0 ? null : selectedPlaterId,
			failureModes: [...selectedFailureModes]
		};

		try {
			const result = id === null
				? await createPrint(selectedProfileId, selectedSpoolId, selectedPrinterId, input)
				: await updatePrint(id, selectedPrinterId, input);

			if (!result.ok) {
				errorMessage = result.error ?? 'Save failed.';
				return;
			}

			if (id === null && matchedJob && !matchDismissed && result.print) {
				await attachJobToPrint(result.print.id, matchedJob.id);
			}

			goto('/prints');
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<h1 class="mb-6 text-2xl font-semibold">{id === null ? 'Log a print' : 'Edit print'}</h1>

<div class="max-w-lg space-y-4">
	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="project-select">Project (optional)</label>
		<Picker
			id="project-select"
			bind:value={selectedProjectId}
			onValueChange={onProjectChanged}
			options={[{ value: 0, label: '-- none --' }, ...projects.map((p) => ({ value: p.id, label: p.fileName }))]}
		/>

		<div class="mt-1 flex items-center gap-2">
			<input type="file" accept=".3mf" onchange={onThreeMfSelected} class="text-sm" />
			{#if uploading}<span class="text-xs text-muted-foreground">Uploading…</span>{/if}
		</div>
		<div class="mt-1 flex items-center gap-2">
			<input bind:value={importUrl} placeholder="or paste a direct .3mf URL" class="flex-1 rounded-md border px-2 py-1 text-xs" />
			<Button variant="outline" size="sm" disabled={uploading || !importUrl.trim()} onclick={fetchFromUrl}>Fetch</Button>
		</div>
		{#if uploadError}<p class="text-xs text-destructive">{uploadError}</p>{/if}

		{#if versionCandidate}
			<div class="mt-1 flex items-center gap-2 rounded-md border bg-muted px-3 py-2 text-xs">
				<span>Looks like a new version of "{versionCandidate.fileName}" — link it?</span>
				<Button variant="outline" size="sm" onclick={linkVersion}>Link</Button>
				<button type="button" onclick={() => (versionCandidate = null)} class="text-muted-foreground hover:text-foreground">Not now</button>
			</div>
		{/if}

		{#if plates.length > 0}
			<label class="mt-2 text-sm font-medium" for="plate-select">Plate</label>
			<Picker
				id="plate-select"
				bind:value={selectedPlaterId}
				options={plates.map((plate) => ({ value: plate.platerId, label: plate.platerName ?? `Plate ${plate.platerId}` }))}
			/>
			<div class="mt-1 flex h-48 items-center justify-center overflow-hidden rounded-md bg-slate-900">
				{#if selectedPlate?.thumbnailBytes}
					<img src="data:image/png;base64,{selectedPlate.thumbnailBytes}" class="h-full w-full object-contain" alt="Plate preview" />
				{:else}
					<span class="text-sm text-slate-400">No plate preview</span>
				{/if}
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="spool-select">Spool</label>
		<Picker
			id="spool-select"
			bind:value={selectedSpoolId}
			onValueChange={onSpoolChanged}
			options={[
				{ value: 0, label: '-- pick a spool --' },
				...spools.map((s) => ({
					value: s.id,
					label: `${s.filament?.brand} ${s.filament?.material} ${s.filament?.variant ?? ''} — ${s.filament?.color}${s.lotCode ? ` (${s.lotCode})` : ''}`
				}))
			]}
		/>
	</div>

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="profile-select">Profile</label>
		<Picker
			id="profile-select"
			bind:value={selectedProfileId}
			options={[{ value: 0, label: '-- pick a profile --' }, ...profiles.map((p) => ({ value: p.id, label: p.name }))]}
		/>
	</div>

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="printer-select">Printer</label>
		<Picker
			id="printer-select"
			bind:value={selectedPrinterId}
			onValueChange={refreshJobMatch}
			options={[{ value: 0, label: '-- pick a printer --' }, ...printers.map((pr) => ({ value: pr.id, label: pr.name }))]}
		/>
	</div>

	{#if matchedJob && !matchDismissed}
		<div class="flex items-center justify-between rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-800">
			<span>Live telemetry found a job started {new Date(matchedJob.startedAt).toLocaleTimeString()} — attach it to this print?</span>
			<button type="button" onclick={() => (matchDismissed = true)} class="ml-3 shrink-0 text-green-700 hover:text-green-900">Dismiss</button>
		</div>
	{/if}

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="started">Started</label>
		<input id="started" type="datetime-local" bind:value={startedAt} onchange={refreshJobMatch} class="rounded-md border px-3 py-2 text-sm" />
	</div>
	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="ended">Ended</label>
		<input id="ended" type="datetime-local" bind:value={endedAt} class="rounded-md border px-3 py-2 text-sm" />
	</div>

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="status-select">Result</label>
		<Picker id="status-select" bind:value={status} options={ALL_STATUSES.map((s) => ({ value: s, label: s }))} />
	</div>

	{#if status === 'Failed' || status === 'Partial'}
		<div class="flex flex-col gap-1">
			<span class="text-sm font-medium">Failure modes</span>
			<div class="flex flex-wrap gap-2">
				{#each ALL_FAILURE_MODES as mode (mode)}
					<label
						class="cursor-pointer rounded-md border px-3 py-1.5 text-sm has-[:checked]:border-foreground has-[:checked]:bg-foreground has-[:checked]:text-background"
					>
						<input
							type="checkbox"
							class="sr-only"
							checked={selectedFailureModes.has(mode)}
							onchange={(e) => toggleFailureMode(mode, e.currentTarget.checked)}
						/>
						{FAILURE_MODE_LABELS[mode]}
					</label>
				{/each}
			</div>
		</div>
	{/if}

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="ams-humidity">AMS humidity (%)</label>
		<input id="ams-humidity" type="number" bind:value={amsHumidityPct} class="rounded-md border px-3 py-2 text-sm" />
	</div>
	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="room-temp">Actual room temp (°C)</label>
		<input id="room-temp" type="number" step="0.1" bind:value={actualRoomTempC} class="rounded-md border px-3 py-2 text-sm" />
	</div>
	<div>
		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={cleanBuildPlate} />
			Clean build plate
		</label>
	</div>

	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium" for="notes">Notes</label>
		<Textarea id="notes" bind:value={notes} class="min-h-[80px]" />
	</div>

	{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

	<form onsubmit={save} class="flex items-center gap-3 pt-2">
		<Button type="submit">Save</Button>
		<a href="/prints" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
	</form>
</div>
