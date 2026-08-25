<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import {
		getGoogleConfig,
		getSettings,
		me,
		saveGoogleConfig,
		saveSettings,
		unlinkGoogle,
		updateAccount
	} from '$lib/api/client';

	let additionalUrls = $state('');
	let catalogUrl = $state('');
	let lastSyncedAt = $state<string | null>(null);
	let savedMessage = $state<string | null>(null);
	let authenticated = $state(false);
	let googleLinked = $state(false);

	let currentPassword = $state('');
	let newUsername = $state('');
	let newPassword = $state('');
	let accountError = $state<string | null>(null);
	let accountSavedMessage = $state<string | null>(null);

	let googleClientId = $state('');
	let googleClientSecret = $state('');
	let googleRedirectUri = $state('');
	let googleSecretSet = $state(false);
	let googleConfigError = $state<string | null>(null);
	let googleConfigSavedMessage = $state<string | null>(null);

	async function saveGoogleSso() {
		googleConfigError = null;
		googleConfigSavedMessage = null;
		const result = await saveGoogleConfig(googleClientId.trim(), googleClientSecret, googleRedirectUri.trim());
		if (!result.ok) {
			googleConfigError = 'Save failed.';
			return;
		}
		googleClientSecret = '';
		googleConfigSavedMessage = 'Saved.';
		const config = await getGoogleConfig();
		googleSecretSet = config.secretSet;
	}

	async function disconnectGoogle() {
		await unlinkGoogle();
		googleLinked = false;
	}

	async function save() {
		await saveSettings(additionalUrls.trim() || null);
		savedMessage = 'Saved.';
	}

	async function saveAccount() {
		accountError = null;
		accountSavedMessage = null;
		const result = await updateAccount(currentPassword, newUsername.trim() || undefined, newPassword || undefined);
		if (!result.ok) {
			accountError = result.error === 'wrong_current_password' ? 'Current password is wrong.' : 'Update failed.';
			return;
		}
		currentPassword = '';
		newUsername = '';
		newPassword = '';
		accountSavedMessage = 'Account updated.';
	}

	$effect(() => {
		getSettings().then((s) => {
			additionalUrls = s.additionalFilamentSourceUrls ?? '';
			catalogUrl = s.catalogUrl;
			lastSyncedAt = s.lastFilamentSyncAt;
		});
		me().then((r) => {
			authenticated = r.authenticated;
			googleLinked = r.googleLinked ?? false;
			if (authenticated) {
				getGoogleConfig().then((c) => {
					googleClientId = c.clientId ?? '';
					googleRedirectUri = c.redirectUri ?? '';
					googleSecretSet = c.secretSet;
				});
			}
		});
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

		{#if authenticated}
			<div class="border-t pt-6">
				<h2 class="mb-3 text-lg font-semibold">Account</h2>
				<div class="space-y-3">
					<Input type="password" bind:value={currentPassword} placeholder="Current password" />
					<Input bind:value={newUsername} placeholder="New username (leave blank to keep current)" />
					<Input type="password" bind:value={newPassword} placeholder="New password (leave blank to keep current)" />
					{#if accountError}<p class="text-sm text-destructive">{accountError}</p>{/if}
					<div>
						<Button onclick={saveAccount} disabled={!currentPassword}>Update account</Button>
						{#if accountSavedMessage}<span class="ml-3 text-sm text-muted-foreground">{accountSavedMessage}</span>{/if}
					</div>
				</div>
			</div>

			<div class="border-t pt-6">
				<h2 class="mb-1 text-lg font-semibold">Single sign-on</h2>
				<p class="mb-3 text-xs text-muted-foreground">
					Register your own OAuth client in the
					<a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noreferrer" class="underline">Google Cloud Console</a>
					and paste its details here to let admins sign in with Google instead of a password.
				</p>
				<div class="space-y-3">
					<Input bind:value={googleClientId} placeholder="Client ID" />
					<Input type="password" bind:value={googleClientSecret} placeholder={googleSecretSet ? 'Client secret (set — leave blank to keep it)' : 'Client secret'} />
					<Input bind:value={googleRedirectUri} placeholder="Redirect URI (e.g. https://spoolbook.example.com/api/auth/google/callback)" />
					{#if googleConfigError}<p class="text-sm text-destructive">{googleConfigError}</p>{/if}
					<div>
						<Button onclick={saveGoogleSso} disabled={!googleClientId || !googleRedirectUri}>Save</Button>
						{#if googleConfigSavedMessage}<span class="ml-3 text-sm text-muted-foreground">{googleConfigSavedMessage}</span>{/if}
					</div>
				</div>

				{#if googleSecretSet}
					<div class="mt-4 border-t pt-4">
						{#if googleLinked}
							<p class="mb-2 text-sm text-muted-foreground">Your account is linked to Google.</p>
							<Button variant="outline" onclick={disconnectGoogle}>Unlink Google account</Button>
						{:else}
							<Button variant="outline" onclick={() => (window.location.href = '/api/auth/google/login')}>Link Google account</Button>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
