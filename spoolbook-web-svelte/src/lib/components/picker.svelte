<script lang="ts" generics="T extends string | number">
	import * as Select from '$lib/components/ui/select/index.js';
	import ColorSwatch from '$lib/components/color-swatch.svelte';

	// Thin wrapper over shadcn's Select (bits-ui) — its Root only accepts a string `value`, so
	// every call site would otherwise repeat the same String()/lookup conversion dance for
	// numeric ids (project/spool/profile/printer pickers) and nullable strings (plate/status
	// pickers) alike. One component instead of ~15 near-identical inline blocks. An option with
	// `hex` shows that filament color's swatch (007/FR-051).
	let {
		value = $bindable(),
		options,
		id,
		placeholder = '',
		disabled = false,
		onValueChange
	}: {
		value: T;
		options: { value: T; label: string; hex?: string | null }[];
		id?: string;
		placeholder?: string;
		disabled?: boolean;
		onValueChange?: (value: T) => void;
	} = $props();

	let stringValue = $derived(String(value));
	let current = $derived(options.find((o) => String(o.value) === stringValue));

	function handleChange(v: string) {
		const match = options.find((o) => String(o.value) === v);
		if (!match) return;
		value = match.value;
		onValueChange?.(match.value);
	}
</script>

<Select.Root type="single" value={stringValue} onValueChange={handleChange} {disabled}>
	<Select.Trigger {id} class="w-full">
		<span class="flex min-w-0 items-center gap-2">
			{#if current?.hex !== undefined}<ColorSwatch hex={current.hex} />{/if}
			<span class="truncate">{current?.label ?? placeholder}</span>
		</span>
	</Select.Trigger>
	<Select.Content>
		{#each options as opt (String(opt.value))}
			<Select.Item value={String(opt.value)} label={opt.label}>
				{#if opt.hex !== undefined}<ColorSwatch hex={opt.hex} />{/if}
				{opt.label}
			</Select.Item>
		{/each}
	</Select.Content>
</Select.Root>
