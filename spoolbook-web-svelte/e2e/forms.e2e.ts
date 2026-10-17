import { expect, test } from '@playwright/test';
import { addFilament, addPrinter, addProfile, addSpool, attachProject, seedPrint, seedProject, unique } from './helpers';

// spec: 002/FR-015
test('add opens a modal over the list, keeps its filters, and saving refreshes the list', async ({ page, request }) => {
	const brand = unique('Brand');
	await addFilament(request, { brand, material: 'PLA', color: 'Black' });
	await page.goto(`/spools?brand=${encodeURIComponent(brand)}`);
	await expect(page.getByText('No spools')).toBeVisible();

	await page.getByText('+ Add spool').click();
	const dialog = page.getByRole('dialog');
	await expect(dialog).toBeVisible();
	await expect(page).toHaveURL(/[?&]new/);
	await expect(page).toHaveURL(/brand=/);

	await dialog.locator('#filament').click();
	await page.getByPlaceholder('Search filaments…').fill(brand);
	await page.getByRole('option').first().click();
	const lot = unique('LOT');
	await dialog.getByLabel('Lot code').fill(lot);
	await dialog.getByRole('button', { name: 'Save' }).click();

	await expect(dialog).toBeHidden();
	await expect(page).not.toHaveURL(/[?&]new/);
	await expect(page).toHaveURL(/brand=/);
	await expect(page.getByRole('row').filter({ hasText: lot })).toHaveCount(1);
});

// spec: 002/FR-015
test('edit lives in the URL: old addresses redirect, reload and back reopen it, Escape closes it', async ({ page, request }) => {
	const brand = unique('Brand');
	const id = await addFilament(request, { brand, material: 'PETG', color: 'Black' });
	const dialog = page.getByRole('dialog');

	await page.goto(`/filaments/edit/${id}`);
	await expect(page).toHaveURL(new RegExp(`/filaments\\?edit=${id}$`));
	await expect(dialog.getByLabel('Brand')).toHaveValue(brand);

	await page.reload();
	await expect(dialog.getByLabel('Brand')).toHaveValue(brand);

	await page.keyboard.press('Escape');
	await expect(dialog).toBeHidden();
	await expect(page).toHaveURL(/\/filaments$/);

	await page.goBack();
	await expect(dialog.getByLabel('Brand')).toHaveValue(brand);

	await page.goto('/printers/new');
	await expect(page).toHaveURL(/\/printers\?new/);
	await expect(dialog.getByRole('heading', { name: 'Add printer' })).toBeVisible();
});

// spec: 002/FR-015
test('every add/edit form opens in the same modal design', async ({ page, request }) => {
	const filament = await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' });
	const spool = await addSpool(request, filament);
	const profile = await addProfile(request, filament, unique('Profile'));
	const print = seedPrint({ profileId: profile, spoolId: spool, printerId: await addPrinter(request, unique('Printer')), status: 'Success', startedAt: '2026-10-05T10:00:00Z' });
	const projectName = `${unique('cube').replace(' ', '-')}.3mf`;
	attachProject(print, seedProject(projectName));

	const forms: [string, string, () => Promise<void>][] = [
		['/filaments?new', 'Add filament', async () => {}],
		['/spools?new', 'Add spool', async () => {}],
		[`/profiles?edit=${profile}`, 'Edit profile', async () => {}],
		['/printers?new', 'Add printer', async () => {}],
		['/filaments/colors', 'Add color', () => page.getByRole('button', { name: '+ Add color' }).click()],
		[`/prints/${print}`, 'Edit outcome', () => page.getByRole('button', { name: 'Edit outcome' }).click()],
		['/projects', 'Rename project', () => page.getByRole('row').filter({ hasText: projectName }).getByRole('button', { name: 'Rename' }).click()],
		['/projects', 'Add project', () => page.getByRole('button', { name: '+ Add project' }).click()]
	];
	for (const [url, title, open] of forms) {
		await page.goto(url);
		await open();
		const dialog = page.getByRole('dialog');
		await expect(dialog.getByRole('heading', { name: title }), url).toBeVisible();
		await expect(dialog, title).toHaveAttribute('data-slot', 'form-modal');
		const footer = dialog.locator('[data-slot="form-modal-footer"]');
		await expect(footer.getByRole('button', { name: 'Cancel' }), title).toBeVisible();
		await footer.getByRole('button', { name: 'Cancel' }).click();
		await expect(dialog, title).toBeHidden();
	}
});
