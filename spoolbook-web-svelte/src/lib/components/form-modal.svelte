<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { cn } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	// The one add/edit modal design (002/FR-015): title, scrolling body, error line, then a footer
	// with any secondary actions on the left and Cancel + the primary action on the right.
	// Cancel, ×, Escape and an outside click all call `onClose`; the modal only closes when the
	// caller stops rendering it, so a close can be refused (unsaved edits, 002/FR-008).
	let {
		title,
		size = 'md',
		error = null,
		submitLabel = 'Save',
		busy = false,
		disabled = false,
		onClose,
		onSubmit,
		children,
		aside,
		actions,
		titleExtra
	}: {
		title: string;
		size?: 'md' | 'wide';
		error?: string | null;
		submitLabel?: string;
		busy?: boolean;
		disabled?: boolean;
		onClose: () => void;
		// No onSubmit → no primary button (e.g. a form that acts as you go).
		onSubmit?: () => void;
		children?: Snippet;
		// Below the form, outside it (e.g. a panel with its own forms).
		aside?: Snippet;
		actions?: Snippet;
		titleExtra?: Snippet;
	} = $props();

	const formId = `form-modal-${Math.random().toString(36).slice(2, 8)}`;
</script>

<Dialog.Root bind:open={() => true, (open) => !open && onClose()}>
	<Dialog.Content
		data-slot="form-modal"
		class={cn(
			'flex max-h-[90svh] flex-col gap-0 overflow-hidden p-0',
			size === 'wide' ? 'sm:max-w-5xl' : 'sm:max-w-lg',
			'max-sm:h-svh max-sm:max-h-svh max-sm:max-w-full max-sm:rounded-none max-sm:ring-0'
		)}
	>
		<Dialog.Header class="border-b px-6 py-4 pr-12">
			<div class="flex flex-wrap items-center gap-2">
				<Dialog.Title class="text-lg font-semibold">{title}</Dialog.Title>
				{@render titleExtra?.()}
			</div>
		</Dialog.Header>

		<div class="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-5">
			<form
				id={formId}
				class="space-y-4"
				onsubmit={(e) => {
					e.preventDefault();
					onSubmit?.();
				}}
			>
				{@render children?.()}
			</form>
			{@render aside?.()}
		</div>

		{#if error}<p class="border-t bg-destructive/5 px-6 py-2.5 text-sm text-destructive">{error}</p>{/if}

		<div data-slot="form-modal-footer" class="flex flex-wrap items-center gap-2 border-t bg-muted/50 px-6 py-3.5">
			{@render actions?.()}
			<div class="ml-auto flex items-center gap-2">
				<Button type="button" variant="ghost" onclick={onClose}>Cancel</Button>
				{#if onSubmit}
					<Button type="submit" form={formId} disabled={busy || disabled}>{submitLabel}</Button>
				{/if}
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
