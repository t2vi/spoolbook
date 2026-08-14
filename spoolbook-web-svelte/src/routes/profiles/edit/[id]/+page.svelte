<script lang="ts">
	import ProfileForm from '$lib/components/profiles/ProfileForm.svelte';
	import { page } from '$app/state';

	let id = $derived(Number(page.params.id));
</script>

<svelte:head>
	<title>Edit profile</title>
</svelte:head>

<div class="mx-auto max-w-4xl px-4 py-8">
	<ProfileForm {id} />
</div>
