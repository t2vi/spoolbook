<script lang="ts" generics="T extends string | number">
	import { Popover as PopoverPrimitive } from 'bits-ui';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ColorSwatch from '$lib/components/color-swatch.svelte';
	import { matchesAllWords } from '$lib/search';

	// A Picker (plain <Select>) for a list too long to usefully scroll -- adds a search box
	// filtering the same flat option list. Extracted from filament-picker.svelte once a second
	// large-list picker (Bambu system presets) needed the same treatment. Every typed word must
	// match; arrows move, Enter picks, Escape closes (007/FR-052). An option with `hex` shows its
	// swatch; `hex: null` an empty one.
	let {
		value = $bindable(),
		options,
		id,
		placeholder = '-- pick --',
		searchPlaceholder = 'Search…',
		onValueChange
	}: {
		value: T;
		options: Option[];
		id?: string;
		placeholder?: string;
		searchPlaceholder?: string;
		onValueChange?: (value: T) => void;
	} = $props();

	type Option = { value: T; label: string; hex?: string | null };
	const LIMIT = 200;

	let open = $state(false);
	let search = $state('');

	let selected = $derived(options.find((o) => o.value === value) ?? null);
	let filtered = $derived.by(() => {
		const q = search.trim();
		return q ? options.filter((o) => matchesAllWords(o.label, q)) : options;
	});
	let shown = $derived(filtered.slice(0, LIMIT));
	let active = $state(0);
	let listEl = $state<HTMLElement | null>(null);

	function onSearchInput() {
		active = 0;
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			if (shown.length === 0) return;
			active = (active + (e.key === 'ArrowDown' ? 1 : shown.length - 1)) % shown.length;
			listEl?.children[active]?.scrollIntoView({ block: 'nearest' });
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (shown[active]) pick(shown[active]);
		}
	}

	function pick(opt: Option) {
		value = opt.value;
		open = false;
		search = '';
		onValueChange?.(opt.value);
	}
</script>

<PopoverPrimitive.Root bind:open>
	<PopoverPrimitive.Trigger
		{id}
		class="flex w-full items-center justify-between gap-1.5 rounded-lg border border-input bg-transparent py-2 pr-2 pl-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
	>
		<span class="flex min-w-0 items-center gap-2 {selected ? '' : 'text-muted-foreground'}">
			{#if selected && selected.hex !== undefined}<ColorSwatch hex={selected.hex} />{/if}
			<span class="truncate">{selected ? selected.label : placeholder}</span>
		</span>
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
				oninput={onSearchInput}
				onkeydown={onKeydown}
				role="combobox"
				aria-expanded={open}
				aria-controls="{id ?? 'picker'}-options"
				aria-activedescendant="{id ?? 'picker'}-option-{active}"
				placeholder={searchPlaceholder}
				autofocus
				class="border-b bg-transparent px-3 py-2 text-sm outline-none placeholder:text-muted-foreground"
			/>
			{#if filtered.length === 0}
				<p class="px-2 py-4 text-center text-sm text-muted-foreground">No matches.</p>
			{:else}
				<div bind:this={listEl} id="{id ?? 'picker'}-options" role="listbox" class="max-h-72 overflow-y-auto p-1">
					{#each shown as opt, i (String(opt.value))}
						<button
							type="button"
							id="{id ?? 'picker'}-option-{i}"
							role="option"
							aria-selected={i === active}
							tabindex="-1"
							onclick={() => pick(opt)}
							onmousemove={() => (active = i)}
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm {i === active ? 'bg-accent text-accent-foreground' : ''}"
						>
							<span class="flex w-3.5 shrink-0 items-center justify-center">
								{#if opt.value === value}<CheckIcon class="size-3.5" />{/if}
							</span>
							{#if opt.hex !== undefined}<ColorSwatch hex={opt.hex} />{/if}
							{opt.label}
						</button>
					{/each}
				</div>
				{#if filtered.length > LIMIT}
					<p class="border-t px-2 py-2 text-center text-xs text-muted-foreground">{filtered.length - LIMIT} more — keep typing to narrow it down.</p>
				{/if}
			{/if}
		</PopoverPrimitive.Content>
	</PopoverPrimitive.Portal>
</PopoverPrimitive.Root>
