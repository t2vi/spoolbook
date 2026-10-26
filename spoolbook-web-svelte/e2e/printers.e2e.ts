import { expect, test } from '@playwright/test';
import { addFilament, addProfile, addSpool, unique } from './helpers';

// spec: 004/FR-050
test('picking an AMS tray sticks through live telemetry ticks and fills spool + profile', async ({ page, request }) => {
	// Unique materials so the tray → spool default matches exactly one spool in the shared DB.
	const petg = unique('PETG');
	const pla = unique('PLA');
	await addSpool(request, await addFilament(request, { brand: 'Slic3D', material: petg, color: 'Black' }));
	const plaFilament = await addFilament(request, { brand: 'Bambu Lab', material: pla, color: 'Yellow' });
	await addSpool(request, plaFilament);
	const profile = unique('Yellow profile');
	await addProfile(request, plaFilament, profile);

	const name = unique('Printer');
	const res = await request.post('/api/printers', {
		// TEST-NET address: enables the Print button, never actually reachable.
		data: { name, model: 'P2S', ipAddress: '192.0.2.1', accessCode: '12345678', serialNumber: 'SN' }
	});
	const { printer } = await res.json();

	// Fake the live feed; `retry` makes EventSource reconnect fast, so a fresh snapshot lands
	// every ~200 ms like real telemetry.
	const snapshot = {
		connected: true,
		cameraStatus: 'NotStarted',
		cameraError: null,
		gcodeState: 'IDLE',
		errors: [],
		amsUnits: [{ unitId: '0', humidityPct: 31, humidityLevel: null, trays: [
			{ slotId: '0', materialType: petg, colorHex: '000000FF', remainPercent: 5 },
			{ slotId: '1', materialType: pla, colorHex: 'F4EE2AFF', remainPercent: 70 }
		] }]
	};
	await page.route(`**/api/printers/${printer.id}/live`, (route) =>
		route.fulfill({ contentType: 'text/event-stream', body: `retry: 200\ndata: ${JSON.stringify(snapshot)}\n\n` })
	);

	await page.goto('/printers');
	const card = page.locator('[data-slot=card]').filter({ has: page.getByRole('heading', { name }) });
	await card.getByRole('button', { name: 'Print', exact: true }).click();
	const dialog = page.getByRole('dialog');

	const slot1 = dialog.getByRole('radio', { name: `AMS 0 slot 1 — ${pla}` });
	await slot1.click();
	await page.waitForTimeout(1000); // several telemetry ticks
	await expect(slot1).toBeChecked();
	await expect(dialog.locator('#spool-select')).toContainText('Yellow');
	await expect(dialog.locator('#profile-select')).toContainText(profile);
});
