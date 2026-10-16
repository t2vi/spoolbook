<script lang="ts">
	import { swatchStyle } from '$lib/color';

	// No known hex → an empty outlined swatch (007/FR-050).
	let { hex, class: className = 'size-4' }: { hex: string | null | undefined; class?: string } = $props();
	let style = $derived(swatchStyle(hex));
</script>

<span
	class="inline-block shrink-0 rounded border align-middle {className}"
	style:background={style?.background ?? 'transparent'}
	style:border-color={style?.borderColor}
	title={hex ?? ''}
></span>
