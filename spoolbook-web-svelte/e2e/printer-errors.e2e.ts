import { expect, test } from '@playwright/test';
import { unique } from './helpers';

// spec: 004/FR-050, 004/FR-014
test('printer errors show the full code, Bambu text and link, and flag internal codes', async ({ page, request }) => {
	const name = unique('Printer');
	const res = await request.post('/api/printers', {
		data: { name, model: 'P2S', ipAddress: null, accessCode: null, serialNumber: null }
	});
	const { printer } = await res.json();

	const link = 'https://e.bambulab.com/index.php?e=0500030000020070&d=22E8AJ5C0700123&s=device_hms&lang=en';
	const snapshot = {
		connected: true,
		cameraStatus: 'NotStarted',
		cameraError: null,
		gcodeState: 'IDLE',
		amsUnits: [],
		errors: [
			{ code: '07FF-8012', message: 'Failed to get AMS mapping table.', blocking: true, internal: false, wikiUrl: link },
			{ code: '0500-0300-0002-0070', message: 'Internal Bambu code 0500-0300-0002-0070 — Bambu publishes no description for it.', blocking: false, internal: true, wikiUrl: link }
		]
	};
	await page.route(`**/api/printers/${printer.id}/live`, (route) =>
		route.fulfill({ contentType: 'text/event-stream', body: `data: ${JSON.stringify(snapshot)}\n\n` })
	);

	await page.goto('/printers');
	const card = page.locator('[data-slot=card]').filter({ has: page.getByRole('heading', { name }) });

	const blocking = card.getByRole('alert').filter({ hasText: 'Printer error 07FF-8012' });
	await expect(blocking.getByText('Failed to get AMS mapping table.')).toBeVisible();
	await expect(blocking.getByText('internal', { exact: true })).toHaveCount(0);

	const warning = card.getByRole('alert').filter({ hasText: 'Printer warning 0500-0300-0002-0070' });
	await expect(warning.getByText('internal', { exact: true })).toBeVisible();
	await expect(warning.getByRole('link', { name: 'Bambu troubleshooting' })).toHaveAttribute('href', link);
});
