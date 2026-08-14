<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { getSettings, me, saveSettings } from '$lib/api/client';

	let additionalUrls = $state('');
	let catalogUrl = $state('');
	let lastSyncedAt = $state<string | null>(null);
	let savedMessage = $state<string | null>(null);
	let authenticated = $state(false);

	async function save() {
		await saveSettings(additionalUrls.trim() || null);
		savedMessage = 'Saved.';
	}

	$effect(() => {
		getSettings().then((s) => {
			additionalUrls = s.additionalFilamentSourceUrls ?? '';
			catalogUrl = s.catalogUrl;
			lastSyncedAt = s.lastFilamentSyncAt;
		});
		me().then((r) => (authenticated = r.authenticated));
	});
</script>

<svelte:head>
	<title>Settings</title>
</svelte:head>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-semibold">Settings</h1>

	{#if !authenticated}
		<p class="text-sm text-muted-foreground">
			<a href="/login" class="underline">Sign in</a> to edit settings.
		</p>
	{/if}

	<div class="max-w-lg space-y-6">
		<div class="flex flex-col gap-1">
			<label class="text-sm font-medium" for="urls">Additional filament catalog sources</label>
			<p class="text-xs text-muted-foreground">One URL per line. Fetched alongside the default catalog on every sync.</p>
			<Textarea id="urls" bind:value={additionalUrls} disabled={!authenticated} placeholder="https://example.com/my-catalog.json" class="min-h-[100px] font-mono text-sm" />
		</div>

		{#if authenticated}
			<div>
				<Button onclick={save}>Save</Button>
				{#if savedMessage}<span class="ml-3 text-sm text-muted-foreground">{savedMessage}</span>{/if}
			</div>
		{/if}

		<div class="border-t pt-6 text-sm text-muted-foreground">
			<div class="flex justify-between py-1"><span>Default catalog source</span><span class="text-foreground">{catalogUrl}</span></div>
			<div class="flex justify-between py-1">
				<span>Catalog last synced</span>
				<span class="text-foreground">{lastSyncedAt ? new Date(lastSyncedAt).toLocaleString() : 'never'}</span>
			</div>
		</div>
	</div>
</div>
