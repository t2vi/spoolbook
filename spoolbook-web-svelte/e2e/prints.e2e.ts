import { expect, test } from '@playwright/test';
import { addFilament, addPrinter, addProfile, addSpool, choose, seedPrint, unique } from './helpers';

async function seed(request: Parameters<typeof addFilament>[0]) {
	const brand = unique('Brand');
	const filament = await addFilament(request, { brand, material: 'PLA', color: 'Black' });
	const lot = unique('LOT');
	const spool = await addSpool(request, filament, { lotCode: lot });
	const profile = await addProfile(request, filament, unique('Profile'));
	const printerA = unique('Printer A');
	const a = await addPrinter(request, printerA);
	const b = await addPrinter(request, unique('Printer B'));
	const ok = seedPrint({ profileId: profile, spoolId: spool, printerId: a, status: 'Success', startedAt: '2026-10-02T10:00:00Z', ambientTempC: 18.5 });
	const failed = seedPrint({ profileId: profile, spoolId: spool, printerId: b, status: 'Failed', startedAt: '2026-10-03T10:00:00Z' });
	return { brand, lot, printerA, ok, failed };
}

// spec: 003/FR-050
test('prints list: brand / printer / status filters in the URL, with Spool and Ambient columns', async ({ page, request }) => {
	const { brand, lot, printerA } = await seed(request);

	await page.goto('/prints');
	await choose(page, 'All brands', brand);
	const rows = page.getByRole('row').filter({ hasText: brand });
	await expect(rows).toHaveCount(2);
	await choose(page, 'All printers', printerA);
	await expect(rows).toHaveCount(1);
	await expect(rows).toContainText(lot);
	await expect(rows).toContainText('18.5 °C');
	await expect(page).toHaveURL(/printerId=\d+/);

	await choose(page, printerA, 'All printers');
	await choose(page, 'All statuses', 'Failed');
	await expect(rows).toHaveCount(1);
	await expect(rows).toContainText('Failed');
	await page.reload();
	await expect(page.getByRole('row').filter({ hasText: brand })).toHaveCount(1);
});

// spec: 003/FR-051, 003/FR-052
test('print detail: edit the outcome of a finished print', async ({ page, request }) => {
	const { failed } = await seed(request);

	await page.goto(`/prints/${failed}`);
	await expect(page.getByText('Print details')).toBeVisible();
	await page.getByRole('button', { name: 'Edit outcome' }).click();
	const dialog = page.getByRole('dialog');
	await dialog.getByLabel('Result').click();
	await page.getByRole('option', { name: 'Partial', exact: true }).click();
	await dialog.getByLabel('Room temp (°C)').fill('19.5');
	await dialog.getByLabel('Notes').fill('stringing on overhangs');
	await dialog.getByRole('button', { name: 'Save' }).click();
	await expect(dialog).toBeHidden();
	await expect(page.getByText('Partial').first()).toBeVisible();
	await expect(page.getByText('stringing on overhangs')).toBeVisible();
});
