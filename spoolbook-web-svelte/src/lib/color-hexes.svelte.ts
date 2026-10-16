import { listFilamentColors } from '$lib/api/client';

// Filaments name their color; the hex lives on the Color (007/FR-051). One shared, lazily loaded
// name → hex map for every list and picker, refreshed by the Colors page after a change.
let hexes = $state<Record<string, string>>({});
let loaded = false;

export function refreshColorHexes() {
	loaded = true;
	listFilamentColors().then((colors) => (hexes = Object.fromEntries(colors.map((c) => [c.name, c.hex]))));
}

export function hexFor(color: string | null | undefined): string | null {
	if (!loaded) refreshColorHexes();
	return color ? (hexes[color] ?? null) : null;
}
