<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { setup, setupStatus } from '$lib/api/client';

	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	// Setup already happened (e.g. this tab was left open from before, or someone hit /setup
	// directly on an already-configured instance) -- nothing to do here, go sign in instead.
	$effect(() => {
		setupStatus().then((s) => {
			if (!s.needsSetup) goto('/login');
		});
	});

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		error = null;
		if (password !== confirmPassword) {
			error = 'Passwords do not match.';
			return;
		}
		submitting = true;
		const result = await setup(username, password);
		submitting = false;
		if (!result.ok) {
			error = result.error === 'password_too_short' ? 'Password must be at least 8 characters.' : 'Setup failed.';
			return;
		}
		goto('/');
	}
</script>

<svelte:head>
	<title>Set up spoolbook</title>
</svelte:head>

<div class="mx-auto max-w-sm px-4 py-16">
	<h1 class="mb-2 text-2xl font-semibold">Welcome to spoolbook</h1>
	<p class="mb-6 text-sm text-muted-foreground">Create the admin account to get started.</p>
	<form class="space-y-4" onsubmit={submit}>
		<Input bind:value={username} placeholder="Username" autofocus />
		<Input type="password" bind:value={password} placeholder="Password (min. 8 characters)" />
		<Input type="password" bind:value={confirmPassword} placeholder="Confirm password" />
		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{/if}
		<Button type="submit" disabled={submitting || !username || !password}>Create account</Button>
	</form>
</div>
