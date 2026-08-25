<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { login } from '$lib/api/client';

	let password = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		submitting = true;
		error = null;
		const result = await login(password);
		submitting = false;
		if (!result.ok) {
			error = 'Wrong password.';
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
		<Input type="password" bind:value={password} placeholder="Admin password" autofocus />
		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{/if}
		<Button type="submit" disabled={submitting || !password}>Sign in</Button>
	</form>
</div>
