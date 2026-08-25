// Display names for the models printerImagePath() below actually recognizes -- also used to
// populate the Model field's <datalist> so typing/picking one gives a value that matches this
// lookup (free text is still allowed for anything not in the Bambu lineup yet).
export const KNOWN_PRINTER_MODELS = ['X1E', 'X1 Carbon', 'P2S', 'P1S', 'P1P', 'A1 mini', 'A1'];

// Model -> bundled product photo, mirroring bambuddy's frontend/src/utils/printer.ts
// getPrinterImage() (P2S shares P1S's chassis, no dedicated asset there either).
// Returns null when nothing matches or no static/img/printers/<file>.png exists yet —
// callers fall back to a generic icon.
export function printerImagePath(model: string | null | undefined): string | null {
	if (!model) return null;
	const m = model.toLowerCase().replace(/\s+/g, '');
	if (m.includes('x1e')) return '/img/printers/x1e.png';
	if (m.includes('x1c') || m.includes('x1carbon') || m.includes('x1')) return '/img/printers/x1c.png';
	if (m.includes('p2s')) return '/img/printers/p1s.png';
	if (m.includes('p1s')) return '/img/printers/p1s.png';
	if (m.includes('p1p')) return '/img/printers/p1p.png';
	if (m.includes('a1mini')) return '/img/printers/a1mini.png';
	if (m.includes('a1')) return '/img/printers/a1.png';
	return null;
}
