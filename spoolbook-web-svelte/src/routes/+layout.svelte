<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { me, logout } from '$lib/api/client';

	let { children } = $props();

	let authenticated = $state(false);

	$effect(() => {
		me().then((r) => (authenticated = r.authenticated));
	});

	async function signOut() {
		await logout();
		authenticated = false;
	}
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<div class="min-h-screen bg-slate-50">
	<nav class="flex items-center gap-1 bg-slate-900 px-6 py-3 text-sm font-medium text-slate-200">
		<span class="mr-4 text-base font-semibold text-white">Spoolbook</span>
		<a href="/" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Dashboard</a>
		<a href="/filaments" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Filaments</a>
		<a href="/spools" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Spools</a>
		<a href="/profiles" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Profiles</a>
		<a href="/prints" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Prints</a>
		<a href="/printers" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Printers</a>
		<a href="/settings" class="rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Settings</a>
		{#if authenticated}
			<button type="button" onclick={signOut} class="ml-auto rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Sign out</button>
		{:else}
			<a href="/login" class="ml-auto rounded-md px-3 py-1.5 hover:bg-slate-800 hover:text-white">Sign in</a>
		{/if}
	</nav>

	<main>
		{@render children()}
	</main>
</div>
