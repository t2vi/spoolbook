<script lang="ts">
	import { unsavedGuard } from '$lib/unsaved.svelte';
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { hexFor } from '$lib/color-hexes.svelte';
	import { Badge, printStatusVariant } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Chart from '$lib/components/ui/chart/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import Picker from '$lib/components/picker.svelte';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import { LineChart } from 'layerchart';
	import { scaleTime } from 'd3-scale';
	import { annotatePrint, getPrint, getProjectPlates, getHourlyWeather, getPrintReadings } from '$lib/api/client';
	import type { PrintOutcome } from '$lib/api/client';
	import type { FailureMode, HourlyWeatherReading, Print, PrintReading, ProjectPlate } from '$lib/api/types';
	import { page } from '$app/state';
	import { formatDateTime } from '$lib/utils.js';

	const weatherChartConfig = {
		tempC: { label: 'Temp (°C)', color: 'var(--chart-1)' },
		humidityPct: { label: 'Humidity (%)', color: 'var(--chart-2)' }
	} satisfies Chart.ChartConfig;

	const telemetryChartConfig = {
		chamberTempC: { label: 'Chamber temp (°C)', color: 'var(--chart-1)' },
		amsHumidityPct: { label: 'AMS humidity (%)', color: 'var(--chart-2)' }
	} satisfies Chart.ChartConfig;

	const layerChartConfig = {
		layerNum: { label: 'Layer', color: 'var(--chart-3)' }
	} satisfies Chart.ChartConfig;

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

	let id = $derived(Number(page.params.id));
	let print = $state<Print | null>(null);
	let plates = $state<ProjectPlate[]>([]);
	let hourlyWeather = $state<HourlyWeatherReading[]>([]);
	let printReadings = $state<PrintReading[]>([]);

	let plate = $derived(plates.find((p) => p.platerId === print?.projectPlaterId) ?? null);
	let weatherChartData = $derived(hourlyWeather.map((r) => ({ ...r, hour: new Date(r.hour) })));
	let readingsChartData = $derived(printReadings.map((r) => ({ ...r, recordedAt: new Date(r.recordedAt) })));
	let lastReading = $derived(printReadings.length > 0 ? printReadings[printReadings.length - 1] : null);
	let chartTab = $state('weather');

	// Outcome annotation (003/FR-052): the only editable part of a Print, once it's finished.
	const OUTCOMES: { value: PrintOutcome['status']; label: string }[] = [
		{ value: 'Success', label: 'Success' },
		{ value: 'Partial', label: 'Partial' },
		{ value: 'Failed', label: 'Failed' }
	];
	let editing = $state(false);
	let draft = $state<PrintOutcome>({ status: 'Success', failureModes: [], notes: null, actualRoomTempC: null, cleanBuildPlate: null });
	let roomTempText = $state('');
	let outcomeError = $state<string | null>(null);
	// Guarded only while the dialog is open (002/FR-008); every way of closing it marks it clean.
	const unsaved = unsavedGuard({ outcome: () => (editing ? [draft, roomTempText] : null) });
	unsaved.markClean();

	function closeEditor() {
		editing = false;
		unsaved.markClean();
	}

	function startEditing() {
		if (!print || print.status === 'InProgress') return;
		draft = {
			status: print.status,
			failureModes: print.failureModes.map((f) => f.mode),
			notes: print.notes,
			actualRoomTempC: print.actualRoomTempC,
			cleanBuildPlate: print.cleanBuildPlate
		};
		roomTempText = print.actualRoomTempC?.toString() ?? '';
		outcomeError = null;
		editing = true;
		unsaved.markClean();
	}

	function toggleMode(mode: FailureMode, on: boolean) {
		draft.failureModes = on ? [...draft.failureModes, mode] : draft.failureModes.filter((m) => m !== mode);
	}

	async function saveOutcome() {
		const roomTemp = roomTempText.trim() === '' ? null : Number(roomTempText);
		if (roomTemp !== null && Number.isNaN(roomTemp)) {
			outcomeError = 'Room temperature must be a number.';
			return;
		}
		const result = await annotatePrint(id, {
			...draft,
			// Failure modes only belong to Failed / Partial (003/FR-011).
			failureModes: draft.status === 'Success' ? [] : draft.failureModes,
			notes: draft.notes?.trim() ? draft.notes : null,
			actualRoomTempC: roomTemp
		});
		if (!result.ok || !result.print) {
			outcomeError = result.error === 'still_in_progress' ? 'This print is still running.' : (result.error ?? 'Save failed.');
			return;
		}
		print = result.print;
		closeEditor();
	}

	$effect(() => {
		(async () => {
			print = await getPrint(id);
			plates = print.project ? await getProjectPlates(print.project.id) : [];
			hourlyWeather = await getHourlyWeather(id);
			printReadings = await getPrintReadings(id);
			if (hourlyWeather.length === 0 && printReadings.length > 0) chartTab = 'telemetry';
		})();
	});
</script>

<svelte:head>
	<title>Print details</title>
</svelte:head>

<div class="mx-auto w-full max-w-5xl px-4 py-8">
	{#if print === null}
		<Card.Root>
			<Card.Header>
				<Skeleton class="h-7 w-40" />
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
				<Card.Title class="text-2xl">Print details</Card.Title>
				<Card.Action class="flex items-center gap-2">
					<Badge variant={printStatusVariant[print.status]}>{print.status}</Badge>
					{#if print.status !== 'InProgress'}
						<Button size="sm" variant="outline" onclick={startEditing}>Edit outcome</Button>
					{/if}
				</Card.Action>
			</Card.Header>

			<Card.Content class="space-y-6">
				{#if print.project}
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Project</span>
						<span class="text-sm">{print.project.fileName}{plate ? ` — ${plate.platerName ?? `Plate ${plate.platerId}`}` : ''}</span>
					</div>
					<Separator />
				{/if}

				<div class="grid grid-cols-1 gap-6 md:grid-cols-[280px_1fr]">
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Bed photo</span>
						{#if print.bedPhotoBase64}
							<div class="flex h-48 items-center justify-center overflow-hidden rounded-lg bg-slate-900">
								<img src="data:image/jpeg;base64,{print.bedPhotoBase64}" class="h-full w-full object-contain" alt="Bed at print end" />
							</div>
						{:else}
							<div class="flex h-48 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
								No bed photo captured
							</div>
						{/if}
					</div>

					<div class="grid grid-cols-2 gap-x-6 gap-y-4 text-sm">
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Spool</span>
							<span class="flex items-center gap-2"><ColorSwatch hex={hexFor(print.spool?.filament?.color)} />{print.spool?.filament?.brand} {print.spool?.filament?.material} {print.spool?.filament?.variant ?? ''} — {print.spool?.filament?.color}{print.spool
									?.lotCode
									? ` (${print.spool.lotCode})`
									: ''}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Profile</span>
							<span>{print.profile?.name}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Printer</span>
							<span>{print.printer?.name}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Clean build plate</span>
							<span>{print.cleanBuildPlate ? 'Yes' : 'No'}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Started</span>
							<span>{formatDateTime(print.startedAt)}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Ended</span>
							<span>{print.endedAt ? formatDateTime(print.endedAt) : '—'}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Room temp</span>
							<span>{print.actualRoomTempC !== null ? `${print.actualRoomTempC}°C` : '—'}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">AMS humidity</span>
							<span>{print.amsHumidityPct !== null ? `${print.amsHumidityPct}%` : '—'}</span>
						</div>
						<div class="flex flex-col gap-1">
							<span class="font-medium text-muted-foreground">Chamber temp</span>
							<span>{print.chamberTempC !== null ? `${print.chamberTempC}°C` : '—'}</span>
						</div>
						{#if print.ambientTempC !== null || print.ambientHumidityPct !== null}
							<div class="col-span-2 flex flex-col gap-1">
								<span class="font-medium text-muted-foreground">Outdoor weather (auto-fetched)</span>
								<span
									>{print.ambientTempC !== null ? `${print.ambientTempC.toFixed(1)}°C` : '—'} / {print.ambientHumidityPct !== null
										? `${print.ambientHumidityPct.toFixed(0)}% humidity`
										: '—'}</span
								>
							</div>
						{/if}
					</div>
				</div>

				{#if hourlyWeather.length > 0 || printReadings.length > 0}
					<Separator />
					<Tabs.Root bind:value={chartTab}>
						<Tabs.List>
							{#if hourlyWeather.length > 0}
								<Tabs.Trigger value="weather">Room weather</Tabs.Trigger>
							{/if}
							{#if printReadings.length > 0}
								<Tabs.Trigger value="telemetry">Chamber & humidity</Tabs.Trigger>
								<Tabs.Trigger value="layer">Layer progress</Tabs.Trigger>
							{/if}
						</Tabs.List>

						{#if hourlyWeather.length > 0}
							<Tabs.Content value="weather">
								<Chart.Container config={weatherChartConfig} class="h-[240px] w-full">
									<LineChart
										data={weatherChartData}
										x="hour"
										xScale={scaleTime()}
										series={[
											{ key: 'tempC', label: weatherChartConfig.tempC.label, color: weatherChartConfig.tempC.color },
											{ key: 'humidityPct', label: weatherChartConfig.humidityPct.label, color: weatherChartConfig.humidityPct.color }
										]}
										props={{ xAxis: { format: (d: Date) => d.toLocaleTimeString([], { hour: 'numeric' }) } }}
									>
										{#snippet tooltip()}
											<Chart.Tooltip labelKey="hour" />
										{/snippet}
									</LineChart>
								</Chart.Container>
							</Tabs.Content>
						{/if}

						{#if printReadings.length > 0}
							<Tabs.Content value="telemetry">
								<Chart.Container config={telemetryChartConfig} class="h-[240px] w-full">
									<LineChart
										data={readingsChartData}
										x="recordedAt"
										xScale={scaleTime()}
										series={[
											{ key: 'chamberTempC', label: telemetryChartConfig.chamberTempC.label, color: telemetryChartConfig.chamberTempC.color },
											{ key: 'amsHumidityPct', label: telemetryChartConfig.amsHumidityPct.label, color: telemetryChartConfig.amsHumidityPct.color }
										]}
										props={{ xAxis: { format: (d: Date) => d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) } }}
									>
										{#snippet tooltip()}
											<Chart.Tooltip labelKey="recordedAt" />
										{/snippet}
									</LineChart>
								</Chart.Container>
							</Tabs.Content>

							<Tabs.Content value="layer">
								{#if lastReading?.totalLayerNum}
									<p class="mb-2 text-sm text-muted-foreground">Layer {lastReading.layerNum ?? '—'} of {lastReading.totalLayerNum}</p>
								{/if}
								<Chart.Container config={layerChartConfig} class="h-[240px] w-full">
									<LineChart
										data={readingsChartData}
										x="recordedAt"
										xScale={scaleTime()}
										series={[{ key: 'layerNum', label: layerChartConfig.layerNum.label, color: layerChartConfig.layerNum.color }]}
										props={{ xAxis: { format: (d: Date) => d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) } }}
									>
										{#snippet tooltip()}
											<Chart.Tooltip labelKey="recordedAt" />
										{/snippet}
									</LineChart>
								</Chart.Container>
							</Tabs.Content>
						{/if}
					</Tabs.Root>
				{/if}

				{#if print.failureModes.length > 0}
					<Separator />
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Failure modes</span>
						<div class="flex flex-wrap gap-2">
							{#each print.failureModes as f (f.mode)}
								<Badge variant="outline">{FAILURE_MODE_LABELS[f.mode]}</Badge>
							{/each}
						</div>
					</div>
				{/if}

				{#if print.notes}
					<Separator />
					<div class="flex flex-col gap-2">
						<span class="text-sm font-medium text-muted-foreground">Notes</span>
						<p class="text-sm whitespace-pre-wrap">{print.notes}</p>
					</div>
				{/if}
			</Card.Content>

			<Card.Footer>
				<Button href="/prints" variant="ghost" size="sm">Back to prints</Button>
			</Card.Footer>
		</Card.Root>
	{/if}
</div>

{#if editing}
	<Dialog.Root open={true} onOpenChange={(open) => !open && closeEditor()}>
		<Dialog.Content class="max-w-md">
			<Dialog.Header>
				<Dialog.Title>Edit outcome</Dialog.Title>
			</Dialog.Header>
			<div class="space-y-4">
				<div class="flex flex-col gap-1">
					<Label for="outcome-status">Result</Label>
					<Picker id="outcome-status" bind:value={draft.status} options={OUTCOMES} />
				</div>
				{#if draft.status !== 'Success'}
					<div class="flex flex-col gap-2">
						<Label>Failure modes</Label>
						<div class="grid grid-cols-2 gap-2">
							{#each Object.entries(FAILURE_MODE_LABELS) as [mode, label] (mode)}
								<label class="flex items-center gap-2 text-sm">
									<Checkbox
										checked={draft.failureModes.includes(mode as FailureMode)}
										onCheckedChange={(on) => toggleMode(mode as FailureMode, on === true)}
									/>
									{label}
								</label>
							{/each}
						</div>
					</div>
				{/if}
				<div class="flex flex-col gap-1">
					<Label for="outcome-room-temp">Room temp (°C)</Label>
					<Input id="outcome-room-temp" inputmode="decimal" placeholder="e.g. 19.5" bind:value={roomTempText} />
				</div>
				<label class="flex items-center gap-2 text-sm">
					<Checkbox checked={draft.cleanBuildPlate === true} onCheckedChange={(on) => (draft.cleanBuildPlate = on === true)} />
					Clean build plate
				</label>
				<div class="flex flex-col gap-1">
					<Label for="outcome-notes">Notes</Label>
					<Textarea id="outcome-notes" rows={4} value={draft.notes ?? ''} oninput={(e) => (draft.notes = e.currentTarget.value)} />
				</div>
				{#if outcomeError}<p class="text-sm text-destructive">{outcomeError}</p>{/if}
			</div>
			<Dialog.Footer>
				<Button variant="ghost" onclick={closeEditor}>Cancel</Button>
				<Button onclick={saveOutcome}>Save</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>
{/if}
