<script lang="ts">
	import { onMount } from 'svelte';
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { hexFor } from '$lib/color-hexes.svelte';
	import FormModal from '$lib/components/form-modal.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as RadioGroup from '$lib/components/ui/radio-group/index.js';
	import Picker from '$lib/components/picker.svelte';
	import ProjectAdder from '$lib/components/projects/ProjectAdder.svelte';
	import {
		amsSlotNumber,
		getProjectPlates,
		listProjects,
		listProfilesForFilament,
		listSpools,
		recommendProfile,
		resliceProject,
		startPrint
	} from '$lib/api/client';
	import type { AmsUnitReading, Printer, PrintProfile, Project, ProjectPlate, Spool } from '$lib/api/types';

	let {
		printer,
		amsUnits,
		onClose,
		onSent
	}: {
		printer: Printer;
		amsUnits: AmsUnitReading[];
		onClose: () => void;
		onSent: () => void;
	} = $props();

	interface AmsTraySlot {
		unitId: string;
		slotId: string;
		materialType: string | null;
		colorHex: string | null;
		key: string;
	}

	let amsTrays = $derived<AmsTraySlot[]>(
		amsUnits.flatMap((unit) =>
			unit.trays.map((t) => ({
				unitId: unit.unitId,
				slotId: t.slotId,
				materialType: t.materialType,
				colorHex: t.colorHex,
				key: `${unit.unitId}:${t.slotId}`
			}))
		)
	);

	let projects = $state<Project[]>([]);
	let spools = $state<Spool[]>([]);
	let profiles = $state<PrintProfile[]>([]);
	let plates = $state<ProjectPlate[]>([]);
	let selectedProjectId = $state(0);
	let selectedPlaterId = $state('');
	let selectedSpoolId = $state(0);
	let selectedProfileId = $state(0);
	let useAms = $state(true);
	let selectedAmsTrayKey = $state<string | null>(null);
	let recommended = $state<PrintProfile | null>(null);
	let sending = $state(false);
	let sendError = $state<string | null>(null);
	let reslicing = $state(false);
	let resliceError = $state<string | null>(null);
	let resliceSucceeded = $state(false);

	// Plate stepping (004/FR-050): ‹ / › move through a multi-plate project, "n / N" shows where.
	let plateIndex = $derived(plates.findIndex((p) => p.platerId === selectedPlaterId));
	function stepPlate(delta: number) {
		const next = plates[plateIndex + delta];
		if (next) selectedPlaterId = next.platerId;
	}
	let selectedPlate = $derived(plates.find((p) => p.platerId === selectedPlaterId) ?? null);
	let selectedAmsSlot = $derived.by(() => {
		const tray = amsTrays.find((t) => t.key === selectedAmsTrayKey);
		return tray ? amsSlotNumber(tray) : null;
	});

	// Once, not $effect: amsTrays changes on every telemetry tick, and re-running this would
	// snap the user's tray/spool pick back to the default.
	onMount(async () => {
		projects = await listProjects();
		spools = await listSpools();
		const defaultTray = amsTrays.find((t) => t.materialType !== null);
		if (defaultTray) onAmsTraySelected(defaultTray.key);
	});

	function onAmsTraySelected(key: string) {
		selectedAmsTrayKey = key;
		const tray = amsTrays.find((t) => t.key === key);
		if (!tray?.materialType) return;

		// Best-effort default: exactly one non-emptied Spool whose Filament matches this tray's
		// material — ambiguous cases leave the dropdown for the user, same as PrintModal.razor.
		const candidates = spools.filter((s) => s.filament?.material?.toLowerCase() === tray.materialType?.toLowerCase());
		if (candidates.length === 1) {
			selectedSpoolId = candidates[0].id;
			onSpoolChanged();
		}
	}

	async function onProjectChanged() {
		const project = projects.find((p) => p.id === selectedProjectId);
		plates = project ? await getProjectPlates(project.id) : [];
		selectedPlaterId = plates[0]?.platerId ?? '';
		recommended = project ? await recommendProfile(project.id) : null;
		if (recommended && profiles.some((p) => p.id === recommended!.id)) selectedProfileId = recommended.id;
	}

	async function onSpoolChanged() {
		const spool = spools.find((s) => s.id === selectedSpoolId);
		profiles = spool ? await listProfilesForFilament(spool.filamentId) : [];
		selectedProfileId = recommended && profiles.some((p) => p.id === recommended!.id) ? recommended.id : (profiles[0]?.id ?? 0);
	}

	async function onProjectAdded(project: Project) {
		projects = projects.some((p) => p.id === project.id) ? projects.map((p) => (p.id === project.id ? project : p)) : [...projects, project];
		selectedProjectId = project.id;
		plates = await getProjectPlates(project.id);
		selectedPlaterId = plates[0]?.platerId ?? '';
		recommended = await recommendProfile(project.id);
		if (recommended && profiles.some((p) => p.id === recommended!.id)) selectedProfileId = recommended.id;
	}

	async function reslice() {
		const project = projects.find((p) => p.id === selectedProjectId);
		const profile = profiles.find((p) => p.id === selectedProfileId);
		if (!project || !profile || !selectedPlaterId) {
			resliceError = 'Pick a project, plate, and profile first.';
			return;
		}

		reslicing = true;
		resliceError = null;
		resliceSucceeded = false;
		const previousPlaterId = selectedPlaterId;

		try {
			const result = await resliceProject(project.id, profile.id);
			if (!result.ok || !result.project) {
				resliceError = result.error ?? 'Re-slice failed.';
				return;
			}

			const resliced = result.project;
			if (!projects.some((p) => p.id === resliced.id)) projects = [...projects, resliced];
			selectedProjectId = resliced.id;
			plates = await getProjectPlates(resliced.id);
			selectedPlaterId = plates.some((p) => p.platerId === previousPlaterId) ? previousPlaterId : (plates[0]?.platerId ?? '');
			resliceSucceeded = true;
		} catch (e) {
			resliceError = e instanceof Error ? e.message : 'Re-slice failed.';
		} finally {
			reslicing = false;
		}
	}

	async function send() {
		if (!selectedProjectId || !selectedPlaterId || !selectedSpoolId || !selectedProfileId) {
			sendError = 'Pick a project, plate, spool, and profile.';
			return;
		}

		sending = true;
		sendError = null;
		try {
			const result = await startPrint(printer.id, {
				projectId: selectedProjectId,
				platerId: selectedPlaterId,
				spoolId: selectedSpoolId,
				profileId: selectedProfileId,
				useAms,
				amsSlot: selectedAmsSlot ?? 0
			});
			if (!result.ok) {
				sendError = result.error ?? 'Send failed.';
				return;
			}
			onSent();
		} catch (e) {
			sendError = e instanceof Error ? e.message : 'Send failed.';
		} finally {
			sending = false;
		}
	}

	let sendDisabled = $derived(
		sending || reslicing || !selectedProjectId || !selectedPlaterId || !selectedSpoolId || !selectedProfileId ||
			(useAms && amsTrays.length > 0 && !selectedAmsTrayKey)
	);
	let resliceDisabled = $derived(reslicing || sending || !selectedProjectId || !selectedPlaterId || !selectedProfileId);
</script>

<FormModal
	title="Print on {printer.name}"
	error={sendError}
	submitLabel={sending ? 'Sending…' : 'Send to printer'}
	disabled={sendDisabled}
	{onClose}
	onSubmit={send}
>
	<!-- The body holds ProjectAdder's own form, so it sits outside the modal's. -->
	{#snippet aside()}
		<div class="space-y-4">
			<div class="flex flex-col gap-1">
				<Label for="project-select">Project</Label>
				<Picker
					id="project-select"
					bind:value={selectedProjectId}
					onValueChange={onProjectChanged}
					options={[{ value: 0, label: '-- none --' }, ...projects.map((p) => ({ value: p.id, label: p.fileName }))]}
				/>

				<div class="mt-1"><ProjectAdder onAdded={onProjectAdded} /></div>

				{#if plates.length > 0}
					<Label for="plate-select" class="mt-2">Plate</Label>
					<Picker
						id="plate-select"
						bind:value={selectedPlaterId}
						options={plates.map((plate) => ({ value: plate.platerId, label: plate.platerName ?? `Plate ${plate.platerId}` }))}
					/>
					<div class="relative mt-1 flex h-40 items-center justify-center overflow-hidden rounded-md bg-slate-900">
						{#if selectedPlate?.thumbnailBytes}
							<img src="data:image/png;base64,{selectedPlate.thumbnailBytes}" class="h-full w-full object-contain" alt="Plate preview" />
						{:else}
							<span class="text-sm text-slate-400">No plate preview</span>
						{/if}
						{#if plates.length > 1}
							<button
								type="button"
								aria-label="Previous plate"
								disabled={plateIndex <= 0}
								onclick={() => stepPlate(-1)}
								class="absolute top-1/2 left-1 -translate-y-1/2 rounded-full bg-black/50 px-2.5 py-1 text-lg text-white hover:bg-black/70 disabled:opacity-30"
							>‹</button>
							<button
								type="button"
								aria-label="Next plate"
								disabled={plateIndex >= plates.length - 1}
								onclick={() => stepPlate(1)}
								class="absolute top-1/2 right-1 -translate-y-1/2 rounded-full bg-black/50 px-2.5 py-1 text-lg text-white hover:bg-black/70 disabled:opacity-30"
							>›</button>
							<span class="absolute bottom-1 left-1/2 -translate-x-1/2 rounded bg-black/50 px-2 text-xs text-white">{plateIndex + 1} / {plates.length}</span>
						{/if}
					</div>
				{/if}
			</div>

			<div>
				<Label>
					<Checkbox bind:checked={useAms} />
					Use AMS
				</Label>
				{#if useAms}
					{#if amsTrays.length === 0}
						<p class="mt-1 text-xs text-muted-foreground">No AMS data yet — printer may not be connected.</p>
					{:else}
						<RadioGroup.Root
							value={selectedAmsTrayKey ?? ''}
							onValueChange={(v) => onAmsTraySelected(v)}
							class="mt-2 gap-1"
						>
							{#each amsTrays as tray (tray.key)}
								<Label>
									<RadioGroup.Item value={tray.key} disabled={tray.materialType === null} />
									{#if tray.colorHex}
										<span
											class="inline-block h-3 w-3 rounded-full border"
											style:background-color={/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(tray.colorHex) ? `#${tray.colorHex}` : '#cccccc'}
										></span>
									{/if}
									<span class={tray.materialType === null ? 'text-muted-foreground' : ''}>
										AMS {tray.unitId} slot {tray.slotId} — {tray.materialType ?? 'Empty'}
									</span>
								</Label>
							{/each}
						</RadioGroup.Root>
					{/if}
				{/if}
			</div>

			<div class="flex flex-col gap-1">
				<Label for="spool-select">Spool</Label>
				<Picker
					id="spool-select"
					bind:value={selectedSpoolId}
					onValueChange={onSpoolChanged}
					options={[
						{ value: 0, label: '-- pick a spool --' },
						...spools.map((s) => ({
							value: s.id,
							label: `${s.filament?.brand} ${s.filament?.material} ${s.filament?.variant ?? ''} — ${s.filament?.color}${s.lotCode ? ` (${s.lotCode})` : ''}`,
							hex: hexFor(s.filament?.color)
						}))
					]}
				/>
			</div>

			<div class="flex flex-col gap-1">
				<Label for="profile-select">Profile</Label>
				<Picker
					id="profile-select"
					bind:value={selectedProfileId}
					options={[{ value: 0, label: '-- pick a profile --' }, ...profiles.map((p) => ({ value: p.id, label: p.name, hex: hexFor(spools.find((s) => s.id === selectedSpoolId)?.filament?.color) }))]}
				/>
			</div>

			{#if resliceError}<p class="text-sm text-destructive">{resliceError}</p>{/if}
			{#if resliceSucceeded}<p class="text-sm text-emerald-600 dark:text-emerald-400">Re-sliced — now sending the freshly-sliced version.</p>{/if}
			<div>
				<Button variant="outline" size="sm" disabled={resliceDisabled} onclick={reslice}>
					{reslicing ? 'Re-slicing…' : 'Re-slice with this profile'}
				</Button>
				<p class="mt-1 text-xs text-muted-foreground">
					Applies the picked Profile's settings to this project and re-slices it before sending — otherwise the project
					sends exactly as originally sliced.
				</p>
			</div>

		</div>
	{/snippet}
</FormModal>
