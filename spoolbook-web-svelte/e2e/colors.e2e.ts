import { expect, test } from '@playwright/test';
import { addFilament, unique } from './helpers';

// spec: 007/FR-042, 007/FR-050
test('colors: add a multi-color, see its split swatch, edit it, and delete is refused while in use', async ({ page, request }) => {
	const name = unique('Gold Black');
	await page.goto('/filaments/colors');
	await page.getByRole('button', { name: '+ Add color' }).click();
	const dialog = page.getByRole('dialog');
	await dialog.getByLabel('Name').fill(name);
	await dialog.getByLabel('Hex').fill('nope');
	await dialog.getByRole('button', { name: 'Save' }).click();
	await expect(dialog.getByText(/must look like #RRGGBB/)).toBeVisible();
	await dialog.getByLabel('Hex').fill('#1A1A1A, #D4AF37');
	await dialog.getByRole('button', { name: 'Save' }).click();
	await expect(dialog).toBeHidden();

	await page.getByPlaceholder('Filter by name').fill(name);
	const row = page.getByRole('row').filter({ hasText: name });
	await expect(row).toHaveCount(1);
	const swatch = row.locator('span[title="#1A1A1A, #D4AF37"]');
	await expect(swatch).toHaveCSS('background-image', /linear-gradient/);

	await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: name });
	await row.getByRole('button', { name: 'Delete' }).click();
	await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();
	await expect(page.getByRole('alertdialog').getByText("Can't delete")).toBeVisible();
	await page.getByRole('alertdialog').getByRole('button', { name: 'OK' }).click();

	await row.getByRole('button', { name: 'Edit' }).click();
	await dialog.getByLabel('Hex').fill('#FF0000');
	await dialog.getByRole('button', { name: 'Save' }).click();
	await expect(page.getByRole('row').filter({ hasText: name }).getByText('#FF0000')).toBeVisible();
});
