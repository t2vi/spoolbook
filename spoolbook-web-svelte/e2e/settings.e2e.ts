import { expect, test } from '@playwright/test';
import { addFilament, unique } from './helpers';

// spec: 002/FR-045
test('settings: export, then import that file — preview counts, confirm, result', async ({ page, request }, testInfo) => {
	await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' });
	await page.goto('/settings');
	const download = page.waitForEvent('download');
	await page.getByRole('button', { name: 'Export data' }).click();
	const file = testInfo.outputPath('export.zip');
	await (await download).saveAs(file);

	await page.locator('#import-file').setInputFiles(file);
	await page.getByRole('button', { name: 'Preview import' }).click();
	await expect(page.getByText(/already exist/).first()).toBeVisible();
	await page.getByRole('button', { name: 'Confirm import' }).click();
	await expect(page.getByText('Import complete.')).toBeVisible();
});
