import { expect, test } from '@playwright/test';
import { addFilament, addPrinter, addProfile, addSpool, choose, seedPrint, unique } from './helpers';

// spec: 006/FR-040
test('profile list: scope and version columns, brand / scope filters, and the delete refusal', async ({ page, request }) => {
	const brand = unique('Brand');
	const filament = await addFilament(request, { brand, material: 'PLA', color: 'Black' });
	const lot = unique('LOT');
	const spool = await addSpool(request, filament, { lotCode: lot });
	const generic = await addProfile(request, filament, unique('Generic'));
	const specific = await addProfile(request, filament, unique('Specific'), '230', spool);
	seedPrint({ profileId: specific, spoolId: spool, printerId: await addPrinter(request, unique('Printer')), status: 'Success', startedAt: '2026-10-01T10:00:00Z' });

	await page.goto('/profiles');
	await choose(page, 'All brands', brand);
	const rows = page.getByRole('row').filter({ hasText: brand });
	await expect(rows).toHaveCount(2);
	await expect(rows.filter({ hasText: 'Generic' })).toContainText('v1');
	await choose(page, 'Any scope', 'Spool-specific');
	await expect(rows).toHaveCount(1);
	await expect(rows).toContainText(lot);

	await rows.getByRole('button', { name: 'Delete' }).click();
	await expect(page.getByText("Can't delete — a Print used this profile version.")).toBeVisible();
	expect(generic).toBeGreaterThan(0);
});

// spec: 006/FR-041, 006/FR-042, 006/FR-043
test('profile versions: save as new, switch back with "Use this version", rename, and duplicate', async ({ page, request }) => {
	const brand = unique('Brand');
	const filament = await addFilament(request, { brand, material: 'PLA', color: 'Black' });
	const name = unique('Cold day');
	const v1 = await addProfile(request, filament, name);

	await page.goto(`/profiles/edit/${v1}`);
	const panel = page.locator('[data-slot="card"]').filter({ has: page.getByText('Versions', { exact: true }) });
	await expect(panel.getByRole('row').filter({ hasText: 'v1' })).toContainText('Selected');
	await page.getByRole('button', { name: 'Save as new version' }).click();
	await expect(page).toHaveURL(/\/profiles$/);

	await page.goto(`/profiles/edit/${v1}`);
	const v1Row = panel.getByRole('row').filter({ hasText: 'v1' });
	const v2Row = panel.getByRole('row').filter({ hasText: 'v2' });
	await expect(v2Row).toContainText('Selected');
	await expect(v1Row).not.toContainText('Selected');
	await v1Row.getByRole('button', { name: 'Use this version' }).click();
	await expect(v1Row).toContainText('Selected');
	await expect(v2Row).not.toContainText('Selected');

	await v1Row.getByRole('button', { name: 'Rename' }).click();
	await v1Row.getByPlaceholder('e.g. Winter 2026').fill('Winter 2026');
	await v1Row.getByRole('button', { name: 'Save' }).click();
	await expect(v1Row).toContainText('Winter 2026');

	await page.getByRole('button', { name: 'Duplicate' }).click();
	await expect(page).toHaveURL(/\/profiles\?edit=\d+$/);
	await expect(page).not.toHaveURL(new RegExp(`edit=${v1}$`));
	await expect(page.getByLabel('Name')).toHaveValue(`${name} (copy)`);
});
