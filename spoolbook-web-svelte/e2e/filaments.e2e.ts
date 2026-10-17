import { expect, test } from '@playwright/test';
import { addSpool, unique } from './helpers';

// spec: 007/FR-040
test('filaments: add, find by brand, edit, and the delete refusal while a spool uses it', async ({ page, request }) => {
	const brand = unique('Brand');
	await page.goto('/filaments/new');
	await page.getByLabel('Brand').fill(brand);
	await page.getByLabel('Material').fill('PETG');
	await page.getByLabel('Variant').fill('HF');
	await page.getByLabel('Color').fill('Black');
	await page.getByRole('button', { name: 'Save' }).click();
	await expect(page).toHaveURL(/\/filaments$/);

	await page.getByPlaceholder('Filter by brand').fill(brand);
	const row = page.getByRole('row').filter({ hasText: brand });
	await expect(row).toHaveCount(1);
	await row.getByRole('link', { name: 'Edit' }).click();
	await page.getByLabel('Variant').fill('Translucent');
	await page.getByRole('button', { name: 'Save' }).click();
	await page.getByPlaceholder('Filter by brand').fill(brand);
	await expect(row).toContainText('Translucent');

	const id = Number((await row.getByRole('link', { name: 'Edit' }).getAttribute('href'))!.split('=').pop());
	await addSpool(request, id);
	await row.getByRole('button', { name: 'Delete' }).click();
	await expect(page.getByText("Can't delete — spools reference this filament.")).toBeVisible();
});
