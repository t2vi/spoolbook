<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import FormModal from '$lib/components/form-modal.svelte';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { createPrinter, getPrinter, testPrinterConnection, updatePrinter } from '$lib/api/client';
	import type { PrinterInput } from '$lib/api/client';
	import { KNOWN_PRINTER_MODELS } from '$lib/printer-image';
	import { unsavedGuard } from '$lib/unsaved.svelte';

	// `onClose(true)` after a save; `onClose(false)` to close without one.
	let { id, onClose }: { id: number | null; onClose: (saved: boolean) => void } = $props();

	// Local form state stays plain strings — shadcn's Input binds HTMLInputAttributes['value']
	// (string), not string | null. Blank -> null happens once, at submit.
	let name = $state('');
	let model = $state('');
	let ipAddress = $state('');
	let accessCode = $state('');
	// Write-only: an existing printer's code is never sent to the browser; blank on save keeps it.
	let hasStoredAccessCode = $state(false);
	let serialNumber = $state('');
	let errorMessage = $state<string | null>(null);
	let testing = $state(false);
	let testResult = $state<{ ok: boolean; error?: string | null } | null>(null);
	const unsaved = unsavedGuard({ form: () => [name, model, ipAddress, accessCode, serialNumber] });

	$effect(() => {
		if (id === null) return unsaved.markClean();
		getPrinter(id).then((existing) => {
			if (!existing) return;
			name = existing.name;
			model = existing.model ?? '';
			ipAddress = existing.ipAddress ?? '';
			hasStoredAccessCode = existing.hasAccessCode;
			serialNumber = existing.serialNumber ?? '';
			unsaved.markClean();
		});
	});

	async function testConnection() {
		if (!ipAddress.trim() || (!accessCode.trim() && !hasStoredAccessCode)) {
			testResult = { ok: false, error: 'Enter an IP address and access code first.' };
			return;
		}

		testing = true;
		testResult = null;
		try {
			testResult = await testPrinterConnection(ipAddress, accessCode, id);
		} finally {
			testing = false;
		}
	}

	async function save() {
		const input: PrinterInput = {
			name,
			model: model.trim() || null,
			ipAddress: ipAddress.trim() || null,
			accessCode: accessCode.trim() || null,
			serialNumber: serialNumber.trim() || null
		};
		try {
			const result = id === null ? await createPrinter(input) : await updatePrinter(id, input);
			if (!result.ok) {
				errorMessage = result.error === 'duplicate' ? 'A printer with this name already exists.' : (result.error ?? 'Save failed.');
				return;
			}
			unsaved.markClean();
			onClose(true);
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<FormModal title={id === null ? 'Add printer' : 'Edit printer'} error={errorMessage} onClose={() => onClose(false)} onSubmit={save}>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
		<div class="flex flex-col gap-1.5">
			<Label for="name">Name</Label>
			<Input id="name" bind:value={name} />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="model">Model</Label>
			<Input id="model" bind:value={model} list="known-printer-models" placeholder="Pick or type a model" />
			<datalist id="known-printer-models">
				{#each KNOWN_PRINTER_MODELS as m (m)}
					<option value={m}></option>
				{/each}
			</datalist>
		</div>
	</div>

	<h3 class="pt-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Live telemetry (optional)</h3>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
		<div class="flex flex-col gap-1.5">
			<Label for="ip">IP address</Label>
			<Input id="ip" bind:value={ipAddress} />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="access-code">Access code</Label>
			<Input id="access-code" bind:value={accessCode} placeholder={hasStoredAccessCode ? 'Saved — leave blank to keep' : ''} />
		</div>
	</div>
	<div class="flex flex-col gap-1.5">
		<Label for="serial">Serial number</Label>
		<Input id="serial" bind:value={serialNumber} />
	</div>

	<div class="flex flex-wrap items-center gap-3">
		<Button type="button" variant="outline" disabled={testing} onclick={testConnection}>
			{testing ? 'Testing…' : 'Test connection'}
		</Button>
		{#if testResult}
			<span class="text-sm {testResult.ok ? 'text-emerald-600 dark:text-emerald-400' : 'text-destructive'}">
				{testResult.ok ? 'Connected successfully.' : `Failed: ${testResult.error}`}
			</span>
		{/if}
	</div>
</FormModal>
