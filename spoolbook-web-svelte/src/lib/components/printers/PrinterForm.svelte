<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { createPrinter, getPrinter, testPrinterConnection, updatePrinter } from '$lib/api/client';
	import type { PrinterInput } from '$lib/api/client';
	import { goto } from '$app/navigation';

	let { id }: { id: number | null } = $props();

	// Local form state stays plain strings — shadcn's Input binds HTMLInputAttributes['value']
	// (string), not string | null. Blank -> null happens once, at submit.
	let name = $state('');
	let model = $state('');
	let ipAddress = $state('');
	let accessCode = $state('');
	let serialNumber = $state('');
	let errorMessage = $state<string | null>(null);
	let testing = $state(false);
	let testResult = $state<{ ok: boolean; error?: string | null } | null>(null);

	$effect(() => {
		if (id === null) return;
		getPrinter(id).then((existing) => {
			if (!existing) return;
			name = existing.name;
			model = existing.model ?? '';
			ipAddress = existing.ipAddress ?? '';
			accessCode = existing.accessCode ?? '';
			serialNumber = existing.serialNumber ?? '';
		});
	});

	async function testConnection() {
		if (!ipAddress.trim() || !accessCode.trim()) {
			testResult = { ok: false, error: 'Enter an IP address and access code first.' };
			return;
		}

		testing = true;
		testResult = null;
		try {
			testResult = await testPrinterConnection(ipAddress, accessCode);
		} finally {
			testing = false;
		}
	}

	async function save(e: Event) {
		e.preventDefault();
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
			goto('/printers');
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : 'Save failed.';
		}
	}
</script>

<h1 class="mb-6 text-2xl font-semibold">{id === null ? 'Add printer' : 'Edit printer'}</h1>

<form onsubmit={save} class="max-w-lg space-y-4">
	<div class="flex flex-col gap-1">
		<Label for="name">Name</Label>
		<Input id="name" bind:value={name} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="model">Model</Label>
		<Input id="model" bind:value={model} />
	</div>

	<h3 class="pt-2 text-sm font-semibold">Live telemetry (optional)</h3>
	<div class="flex flex-col gap-1">
		<Label for="ip">IP address</Label>
		<Input id="ip" bind:value={ipAddress} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="access-code">Access code</Label>
		<Input id="access-code" bind:value={accessCode} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="serial">Serial number</Label>
		<Input id="serial" bind:value={serialNumber} />
	</div>

	<div class="flex items-center gap-3">
		<Button type="button" variant="outline" disabled={testing} onclick={testConnection}>
			{testing ? 'Testing…' : 'Test connection'}
		</Button>
		{#if testResult}
			<span class="text-sm {testResult.ok ? 'text-green-700' : 'text-destructive'}">
				{testResult.ok ? 'Connected successfully.' : `Failed: ${testResult.error}`}
			</span>
		{/if}
	</div>

	{#if errorMessage}<p class="text-sm text-destructive">{errorMessage}</p>{/if}

	<div class="flex items-center gap-3 pt-2">
		<Button type="submit">Save</Button>
		<a href="/printers" class="text-sm text-muted-foreground hover:text-foreground">Cancel</a>
	</div>
</form>
