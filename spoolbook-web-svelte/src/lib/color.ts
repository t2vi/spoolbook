// Swatch rendering (007/FR-050). A color's hex is one #RRGGBB or a comma-separated list for a
// multi-color filament: one fills the swatch solid; several split it into equal diagonal wedges
// from top-left to bottom-right with hard edges, in order (black + gold = two flat halves).
const PART = /^#[0-9a-f]{6}$/i;

export function hexParts(hex: string | null | undefined): string[] | null {
	const parts = (hex ?? '').split(',').map((p) => p.trim()).filter(Boolean);
	return parts.length > 0 && parts.every((p) => PART.test(p)) ? parts : null;
}

export function swatchStyle(hex: string | null | undefined): { background: string; borderColor: string } | null {
	const parts = hexParts(hex);
	if (!parts) return null;
	const background =
		parts.length === 1
			? parts[0]
			: `linear-gradient(to bottom right, ${parts.map((c, i) => `${c} ${(i / parts.length) * 100}% ${((i + 1) / parts.length) * 100}%`).join(', ')})`;
	// Light border on dark fills so near-black swatches stay visible.
	const luminance =
		parts
			.map((c) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16)))
			.reduce((sum, [r, g, b]) => sum + (0.299 * r + 0.587 * g + 0.114 * b) / 255, 0) / parts.length;
	return { background, borderColor: luminance < 0.3 ? '#cbd5e1' : '#64748b' };
}
