<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import PrintModal from './PrintModal.svelte';
	import Link2 from '@lucide/svelte/icons/link-2';
	import Link2Off from '@lucide/svelte/icons/link-2-off';
	import Camera from '@lucide/svelte/icons/camera';
	import CameraOff from '@lucide/svelte/icons/camera-off';
	import Printer from '@lucide/svelte/icons/printer';
	import Droplets from '@lucide/svelte/icons/droplets';
	import { printerImagePath } from '$lib/printer-image';
	import {
		controlPrinter,
		deletePrinter,
		listRecentPrints,
		retryPrinterCamera,
		subscribeToPrinterLiveStatus
	} from '$lib/api/client';
	import type { AmsUnitReading, CameraStatus, Print, Printer as PrinterEntity, PrintStatus } from '$lib/api/types';
	import { formatDateTime } from '$lib/utils.js';

	let {
		printer,
		authenticated,
		onDeleted
	}: {
		printer: PrinterEntity;
		authenticated: boolean;
		onDeleted: () => void;
	} = $props();

	let connected = $state(false);
	let amsUnits = $state<AmsUnitReading[]>([]);
	let cameraStatus = $state<CameraStatus>('NotStarted');
	let cameraError = $state<string | null>(null);
	let gcodeState = $state<string | null>(null);
	let recentPrints = $state<Print[]>([]);
	let sending = $state(false);
	let controlErr = $state<string | null>(null);
	let showPrintModal = $state(false);
	let cameraRetryCount = $state(0);

	let cameraSrc = $derived(`/printers/${printer.id}/camera?r=${cameraRetryCount}`);
	let hasTelemetryConfig = $derived(printer.ipAddress !== null && printer.accessCode !== null);
	// Bambu's gcode_state: RUNNING/PREPARE while actively printing, PAUSE while paused —
	// mirrors BambuMqttPayloadParser.IsActiveState on the backend.
	let isPrinting = $derived(gcodeState === 'RUNNING' || gcodeState === 'PREPARE');
	let isPaused = $derived(gcodeState === 'PAUSE');
	let printerImage = $derived(printerImagePath(printer.model));
	let printerImageFailed = $state(false);
	$effect(() => {
		printerImage;
		printerImageFailed = false;
	});

	$effect(() => subscribeToPrinterLiveStatus(printer.id, (snap) => {
		connected = snap.connected;
		amsUnits = snap.amsUnits;
		cameraStatus = snap.cameraStatus;
		cameraError = snap.cameraError;
		gcodeState = snap.gcodeState;
	}));

	async function refreshRecentPrints() {
		recentPrints = await listRecentPrints(printer.id);
	}
	$effect(() => {
		refreshRecentPrints();
	});

	async function send(command: 'pause' | 'resume' | 'stop') {
		sending = true;
		controlErr = null;
		try {
			const result = await controlPrinter(printer.id, command);
			if (!result.ok) controlErr = result.error ?? 'Failed.';
		} catch (e) {
			controlErr = e instanceof Error ? e.message : 'Failed.';
		} finally {
			sending = false;
		}
	}

	async function remove() {
		if (!authenticated) {
			controlErr = 'Sign in to delete.';
			return;
		}
		try {
			const result = await deletePrinter(printer.id);
			if (result.ok) onDeleted();
			else controlErr = result.error === 'has_prints' ? "Can't delete — prints reference this printer." : (result.error ?? 'Failed.');
		} catch (e) {
			controlErr = e instanceof Error ? e.message : 'Failed.';
		}
	}

	function openCameraWindow() {
		window.open(cameraSrc, '_blank', 'width=960,height=720,noopener');
	}

	async function retryCamera() {
		await retryPrinterCamera(printer.id);
		cameraRetryCount++;
	}

	function onPrintSent() {
		showPrintModal = false;
		refreshRecentPrints();
	}

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

	function trayFill(colorHex: string | null): string | undefined {
		return colorHex ? `#${colorHex.length >= 6 ? colorHex.slice(0, 6) : 'cccccc'}` : undefined;
	}

	// Thresholds match maziggy/bambuddy's own AmsUnitCard defaults (goodThreshold=40,
	// fairThreshold=60) -- reused rather than picked arbitrarily.
	function humidityPctClass(pct: number): string {
		if (pct <= 40) return 'text-green-600';
		if (pct <= 60) return 'text-amber-600';
		return 'text-red-600';
	}

	// Older AMS units have no hygrometer, so humidityPct is null and this 1-5 index (the physical
	// unit's own LED ring, not a percentage) is all that's available -- 1 is driest, 5 is most
	// humid. Shown as "n/5" rather than a bare number so it doesn't read as a percentage.
	function humidityLevelClass(level: number): string {
		if (level <= 2) return 'text-green-600';
		if (level === 3) return 'text-amber-600';
		return 'text-red-600';
	}
</script>

<Card.Root class="gap-4 p-5">
	<div class="flex items-start gap-3">
		<div class="flex h-14 w-14 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-slate-900 text-slate-300">
			{#if printerImage && !printerImageFailed}
				<img
					src={printerImage}
					alt={printer.model ?? printer.name}
					class="h-full w-full bg-white object-contain"
					onerror={() => (printerImageFailed = true)}
				/>
			{:else}
				<Printer class="h-6 w-6" />
			{/if}
		</div>
		<div class="min-w-0 flex-1">
			<div class="flex items-start justify-between gap-2">
				<h2 class="truncate text-xl font-semibold">{printer.name}</h2>
				{#if authenticated}
					<div class="flex shrink-0 items-center gap-3 text-sm">
						<a href="/printers/edit/{printer.id}" class="text-muted-foreground hover:text-foreground hover:underline">Edit</a>
						<button onclick={remove} class="text-destructive hover:underline">Delete</button>
					</div>
				{/if}
			</div>
			{#if printer.model}<p class="text-sm text-muted-foreground">{printer.model}</p>{/if}
			<div class="mt-2 flex flex-wrap gap-2">
				<Badge class={connected ? 'bg-green-100 text-green-800 hover:bg-green-100' : 'bg-slate-100 text-slate-600 hover:bg-slate-100'}>
					{#if connected}<Link2 />{:else}<Link2Off />{/if}
					{connected ? 'Connected' : 'Not connected'}
				</Badge>
				{#if hasTelemetryConfig}
					<Badge
						class={cameraStatus === 'Streaming'
							? 'bg-green-100 text-green-800 hover:bg-green-100'
							: cameraStatus === 'Connecting'
								? 'bg-amber-100 text-amber-800 hover:bg-amber-100'
								: cameraStatus === 'Unavailable'
									? 'bg-red-100 text-red-800 hover:bg-red-100'
									: 'bg-slate-100 text-slate-600 hover:bg-slate-100'}
					>
						{#if cameraStatus === 'Unavailable'}<CameraOff />{:else}<Camera />{/if}
						{cameraStatus === 'Streaming' ? 'Live' : cameraStatus === 'Connecting' ? 'Connecting' : cameraStatus === 'Unavailable' ? 'Camera error' : 'No camera'}
					</Badge>
				{/if}
			</div>
		</div>
	</div>

	<div>
		<h3 class="mb-2 flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase after:h-px after:flex-1 after:bg-border">
			Status
		</h3>
		<div class="flex gap-4 rounded-lg bg-muted p-4">
			<!-- No inline live feed here: an <img> streaming the camera is a permanent client, and
			     Bambu firmware only serves one at a time, so the card holding it starved the
			     printer's own toolhead-camera init mid-print. The camera is popup-only now. -->
			<button
				onclick={() => hasTelemetryConfig && openCameraWindow()}
				class="flex h-20 w-28 shrink-0 items-center justify-center overflow-hidden rounded-md bg-muted-foreground/10 disabled:cursor-default"
				disabled={!hasTelemetryConfig}
				title="View camera"
			>
				<Printer class="h-8 w-8 text-muted-foreground/50" />
			</button>
			<div class="min-w-0 flex-1 text-sm">
				<p class="font-medium">{connected ? 'Connected' : 'Not connected'}</p>
				{#if cameraStatus === 'Unavailable' && cameraError}
					<p class="mt-1 text-destructive">{cameraError}</p>
					<Button variant="outline" size="sm" class="mt-2" onclick={retryCamera}>Retry camera</Button>
				{:else}
					<p class="mt-1 text-muted-foreground">
						{hasTelemetryConfig ? 'No live job data yet.' : 'No telemetry configured.'}
					</p>
				{/if}
			</div>
		</div>
	</div>

	<div>
		<h3 class="mb-2 flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase after:h-px after:flex-1 after:bg-border">
			Controls
		</h3>
		<div class="flex items-center gap-2">
			<Button variant="outline" size="sm" disabled={!connected || sending || !isPrinting} onclick={() => send('pause')}>Pause</Button>
			<Button variant="outline" size="sm" disabled={!connected || sending || !isPaused} onclick={() => send('resume')}>Resume</Button>
			<Button variant="destructive" size="sm" disabled={!connected || sending || !(isPrinting || isPaused)} onclick={() => send('stop')}>Stop</Button>
			<Button variant="outline" size="sm" disabled={!hasTelemetryConfig} onclick={openCameraWindow}>
				<Camera /> View camera
			</Button>
			<Button
				class="ml-auto bg-green-600 hover:bg-green-700"
				size="sm"
				disabled={printer.ipAddress === null || printer.accessCode === null || isPrinting || isPaused}
				onclick={() => (showPrintModal = true)}
			>
				<Printer /> Print
			</Button>
		</div>
		{#if controlErr}<p class="mt-2 text-sm text-destructive">{controlErr}</p>{/if}
	</div>

	<div>
		<h3 class="mb-2 flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase after:h-px after:flex-1 after:bg-border">
			Filaments
		</h3>
		{#if amsUnits.length === 0}
			<p class="text-sm text-muted-foreground">No AMS data yet — waiting for the printer to report, or none installed.</p>
		{:else}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				{#each amsUnits as unit (unit.unitId)}
					<div class="rounded-lg border p-3">
						<div class="mb-2 flex items-center justify-between">
							<span class="text-sm font-medium">AMS-{unit.unitId}</span>
							{#if unit.humidityPct !== null}
								<span class="flex items-center gap-1 text-xs {humidityPctClass(unit.humidityPct)}" title="AMS humidity {unit.humidityPct}%">
									<Droplets class="h-3 w-3" />{unit.humidityPct}%
								</span>
							{:else if unit.humidityLevel !== null}
								<span
									class="flex items-center gap-1 text-xs {humidityLevelClass(unit.humidityLevel)}"
									title="AMS humidity level {unit.humidityLevel}/5 (1 = driest, 5 = most humid)"
								>
									<Droplets class="h-3 w-3" />{unit.humidityLevel}/5
								</span>
							{/if}
						</div>
						<div class="grid grid-cols-4 gap-2">
							{#each unit.trays as tray (tray.slotId)}
								<div class="text-center">
									<div
										class="mx-auto flex h-9 w-9 items-center justify-center rounded-full {tray.materialType === null ? 'border-2 border-dashed border-muted-foreground/40' : 'border'}"
										style:background-color={tray.materialType === null ? undefined : trayFill(tray.colorHex)}
									></div>
									<p class="mt-1 truncate text-xs {tray.materialType === null ? 'text-muted-foreground' : 'font-medium'}">
										{tray.materialType ?? 'Empty'}
									</p>
									<div class="mt-1 h-1 rounded-full bg-muted-foreground/15">
										<div class="h-1 rounded-full bg-foreground/60" style:width="{tray.remainPercent ?? 0}%"></div>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div>
		<h3 class="mb-2 flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase after:h-px after:flex-1 after:bg-border">
			Recent prints
		</h3>
		{#if recentPrints.length === 0}
			<p class="text-sm text-muted-foreground">No prints logged yet.</p>
		{:else}
			<ul class="divide-y rounded-lg border">
				{#each recentPrints as p (p.id)}
					<li class="flex items-center justify-between px-3 py-2 text-sm">
						<span>{formatDateTime(p.startedAt)} — {p.spool?.filament?.brand} {p.spool?.filament?.material}</span>
						<div class="flex items-center gap-3">
							<Badge class={statusBadgeClass(p.status)}>{p.status}</Badge>
							<a href="/prints/{p.id}" class="text-muted-foreground hover:text-foreground hover:underline">View</a>
						</div>
					</li>
				{/each}
			</ul>
			<a href="/prints?printerId={printer.id}" class="mt-2 inline-block text-sm text-muted-foreground hover:text-foreground hover:underline">
				View all prints for this printer
			</a>
		{/if}
	</div>
</Card.Root>

{#if showPrintModal}
	<PrintModal {printer} {amsUnits} onClose={() => (showPrintModal = false)} onSent={onPrintSent} />
{/if}
