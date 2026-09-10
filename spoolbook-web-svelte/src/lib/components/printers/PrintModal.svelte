<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as RadioGroup from '$lib/components/ui/radio-group/index.js';
	import Picker from '$lib/components/picker.svelte';
	import {
		amsSlotNumber,
		getProjectPlates,
		listProjects,
		listProfilesForFilament,
		listSpools,
		recommendProfile,
		resliceProject,
		startPrint,
		uploadProject
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
	let uploading = $state(false);
	let uploadError = $state<string | null>(null);
	let recommended = $state<PrintProfile | null>(null);
	let sending = $state(false);
	let sendError = $state<string | null>(null);
	let reslicing = $state(false);
	let resliceError = $state<string | null>(null);
	let resliceSucceeded = $state(false);

	let selectedPlate = $derived(plates.find((p) => p.platerId === selectedPlaterId) ?? null);
	let selectedAmsSlot = $derived.by(() => {
		const tray = amsTrays.find((t) => t.key === selectedAmsTrayKey);
		return tray ? amsSlotNumber(tray) : null;
	});

	$effect(() => {
		(async () => {
			projects = await listProjects();
			spools = await listSpools();
		})();

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

	async function applyUploadResult(result: { ok: boolean; error?: string | null; project?: Project | null }) {
		if (!result.ok || !result.project) {
			uploadError = `Import failed: ${result.error}`;
			return;
		}
		const project = result.project;
		if (!projects.some((p) => p.id === project.id)) projects = [...projects, project];

		selectedProjectId = project.id;
		plates = await getProjectPlates(project.id);
		selectedPlaterId = plates[0]?.platerId ?? '';
		recommended = await recommendProfile(project.id);
		if (recommended && profiles.some((p) => p.id === recommended!.id)) selectedProfileId = recommended.id;
	}

	async function onThreeMfSelected(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;

		uploading = true;
		uploadError = null;
		try {
			await applyUploadResult(await uploadProject(file));
		} catch (err) {
			uploadError = `Import failed: ${err instanceof Error ? err.message : 'unknown error'}`;
		} finally {
			uploading = false;
		}
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

<Dialog.Root open={true} onOpenChange={(open) => !open && onClose()}>
	<Dialog.Content class="max-h-[90vh] max-w-lg overflow-y-auto sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>Print on {printer.name}</Dialog.Title>
		</Dialog.Header>

		<div class="space-y-4">
			<div class="flex flex-col gap-1">
				<Label for="project-select">Project</Label>
				<Picker
					id="project-select"
					bind:value={selectedProjectId}
					onValueChange={onProjectChanged}
					options={[{ value: 0, label: '-- none --' }, ...projects.map((p) => ({ value: p.id, label: p.fileName }))]}
				/>

				<div class="mt-1 flex items-center gap-2">
					<input
						type="file"
						accept=".3mf"
						onchange={onThreeMfSelected}
						class="text-sm file:mr-3 file:cursor-pointer file:rounded-md file:border file:bg-background file:px-3 file:py-1.5 file:text-sm file:font-medium hover:file:bg-accent"
					/>
					{#if uploading}<span class="text-xs text-muted-foreground">Uploading…</span>{/if}
				</div>
				{#if uploadError}<p class="text-xs text-destructive">{uploadError}</p>{/if}

				{#if plates.length > 0}
					<Label for="plate-select" class="mt-2">Plate</Label>
					<Picker
						id="plate-select"
						bind:value={selectedPlaterId}
						options={plates.map((plate) => ({ value: plate.platerId, label: plate.platerName ?? `Plate ${plate.platerId}` }))}
					/>
					<div class="mt-1 flex h-40 items-center justify-center overflow-hidden rounded-md bg-slate-900">
						{#if selectedPlate?.thumbnailBytes}
							<img src="data:image/png;base64,{selectedPlate.thumbnailBytes}" class="h-full w-full object-contain" alt="Plate preview" />
						{:else}
							<span class="text-sm text-slate-400">No plate preview</span>
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
							label: `${s.filament?.brand} ${s.filament?.material} ${s.filament?.variant ?? ''} — ${s.filament?.color}${s.lotCode ? ` (${s.lotCode})` : ''}`
						}))
					]}
				/>
			</div>

			<div class="flex flex-col gap-1">
				<Label for="profile-select">Profile</Label>
				<Picker
					id="profile-select"
					bind:value={selectedProfileId}
					options={[{ value: 0, label: '-- pick a profile --' }, ...profiles.map((p) => ({ value: p.id, label: p.name }))]}
				/>
			</div>

			{#if resliceError}<p class="text-sm text-destructive">{resliceError}</p>{/if}
			{#if resliceSucceeded}<p class="text-sm text-green-700">Re-sliced — now sending the freshly-sliced version.</p>{/if}
			<div>
				<Button variant="outline" size="sm" disabled={resliceDisabled} onclick={reslice}>
					{reslicing ? 'Re-slicing…' : 'Re-slice with this profile'}
				</Button>
				<p class="mt-1 text-xs text-muted-foreground">
					Applies the picked Profile's settings to this project and re-slices it before sending — otherwise the project
					sends exactly as originally sliced.
				</p>
			</div>

			{#if sendError}<p class="text-sm text-destructive">{sendError}</p>{/if}

			<div class="flex items-center gap-3 pt-2">
				<Button disabled={sendDisabled} onclick={send}>{sending ? 'Sending…' : 'Send to printer'}</Button>
				<Button variant="ghost" onclick={onClose}>Cancel</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
