<script lang="ts">
	import { Popover as PopoverPrimitive } from 'bits-ui';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import type { Filament } from '$lib/api/types';

	// Filament.Picker only -- the catalog runs into the thousands of rows (brand/material/variant/
	// color), so a plain <Select> with every option flattened into one scrolling list (what Picker
	// renders) has no way to find one without scrolling past hundreds of others first. A search box
	// filtering the same flat list fixes that without needing a full generic combobox system --
	// nothing else in this app picks from a list this large.
	let {
		value = $bindable(),
		filaments,
		id,
		placeholder = '-- pick a filament --'
	}: {
		value: number;
		filaments: Filament[];
		id?: string;
		placeholder?: string;
	} = $props();

	let open = $state(false);
	let search = $state('');

	function label(f: Filament): string {
		return `${f.brand} ${f.material} ${f.variant ?? ''} — ${f.color}`;
	}

	let selected = $derived(filaments.find((f) => f.id === value) ?? null);
	let filtered = $derived.by(() => {
		const q = search.trim().toLowerCase();
		if (!q) return filaments;
		return filaments.filter((f) => label(f).toLowerCase().includes(q));
	});

	function pick(f: Filament) {
		value = f.id;
		open = false;
		search = '';
	}
</script>

<PopoverPrimitive.Root bind:open>
	<PopoverPrimitive.Trigger
		{id}
		class="flex w-full items-center justify-between gap-1.5 rounded-lg border border-input bg-transparent py-2 pr-2 pl-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
	>
		<span class={selected ? '' : 'text-muted-foreground'}>{selected ? label(selected) : placeholder}</span>
		<ChevronDownIcon class="size-4 shrink-0 text-muted-foreground" />
	</PopoverPrimitive.Trigger>
	<PopoverPrimitive.Portal>
		<PopoverPrimitive.Content
			sideOffset={4}
			class="z-50 flex w-(--bits-popover-anchor-width) flex-col overflow-hidden rounded-lg bg-popover text-popover-foreground shadow-md ring-1 ring-foreground/10"
		>
			<!-- svelte-ignore a11y_autofocus -- opens only on explicit click, not page load -->
			<input
				bind:value={search}
				placeholder="Search filaments…"
				autofocus
				class="border-b bg-transparent px-3 py-2 text-sm outline-none placeholder:text-muted-foreground"
			/>
			<div class="max-h-72 overflow-y-auto p-1">
				{#if filtered.length === 0}
					<p class="px-2 py-4 text-center text-sm text-muted-foreground">No matching filaments.</p>
				{:else}
					{#each filtered.slice(0, 200) as f (f.id)}
						<button
							type="button"
							onclick={() => pick(f)}
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground"
						>
							<span class="flex w-3.5 shrink-0 items-center justify-center">
								{#if f.id === value}<CheckIcon class="size-3.5" />{/if}
							</span>
							{label(f)}
						</button>
					{/each}
					{#if filtered.length > 200}
						<p class="px-2 py-2 text-center text-xs text-muted-foreground">
							{filtered.length - 200} more — keep typing to narrow it down.
						</p>
					{/if}
				{/if}
			</div>
		</PopoverPrimitive.Content>
	</PopoverPrimitive.Portal>
</PopoverPrimitive.Root>
