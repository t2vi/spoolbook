<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { login, setupStatus, googleStatus } from '$lib/api/client';

	let username = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let googleConfigured = $state(false);

	// No account exists yet at all -- send to the wizard instead of a login form with nothing to
	// log into.
	$effect(() => {
		setupStatus().then((s) => {
			if (s.needsSetup) goto('/setup');
		});
		googleStatus().then((s) => (googleConfigured = s.configured));
	});

	const googleError: Record<string, string> = {
		google_not_linked: 'That Google account isn’t linked to this spoolbook instance.',
		google_failed: 'Google sign-in failed. Try again.',
		google_expired: 'That sign-in attempt expired. Try again.'
	};
	const googleErrorCode = page.url.searchParams.get('error');

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		submitting = true;
		error = null;
		const result = await login(username, password);
		submitting = false;
		if (!result.ok) {
			error = 'Wrong username or password.';
			return;
		}
		goto('/');
	}
</script>

<svelte:head>
	<title>Sign in</title>
</svelte:head>

<div class="mx-auto max-w-sm px-4 py-16">
	<h1 class="mb-6 text-2xl font-semibold">Sign in</h1>
	<form class="space-y-4" onsubmit={submit}>
		<Input bind:value={username} placeholder="Username" autofocus />
		<Input type="password" bind:value={password} placeholder="Password" />
		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{/if}
		{#if googleErrorCode && googleError[googleErrorCode]}
			<p class="text-sm text-destructive">{googleError[googleErrorCode]}</p>
		{/if}
		<Button type="submit" disabled={submitting || !username || !password}>Sign in</Button>
	</form>

	{#if googleConfigured}
		<div class="mt-4 border-t pt-4">
			<Button variant="outline" class="w-full" onclick={() => (window.location.href = '/api/auth/google/login')}>
				Sign in with Google
			</Button>
		</div>
	{/if}
</div>
