import { expect, test } from '@playwright/test';
import { addColor, addFilament, addProfile, addSpool, choose, unique } from './helpers';

// spec: 007/FR-052, 007/FR-051, 007/FR-041
test('spool form: the filament picker matches every typed word, shows swatches and works from the keyboard', async ({ page, request }) => {
	const color = unique('Teal');
	const token = color.split(' ')[1];
	await addColor(request, color, '#008080');
	const brand = unique('Brand');
	await addFilament(request, { brand, material: 'PETG', variant: 'HF', color });
	await addFilament(request, { brand, material: 'PLA', variant: 'Basic', color });

	await page.goto('/spools/new');
	await page.locator('#filament').click();
	const search = page.getByPlaceholder('Search filaments…');
	await search.fill(`hf ${token} PETG`);
	await expect(page.getByRole('option')).toHaveCount(1);
	await expect(page.getByRole('option').locator('span[title="#008080"]')).toBeVisible();
	await search.fill(token);
	await expect(page.getByRole('option')).toHaveCount(2);
	await search.press('ArrowDown');
	await search.press('Enter');
	await expect(page.locator('#filament')).toContainText(/PETG HF|PLA Basic/);
	await page.locator('#filament').click();
	await page.getByPlaceholder('Search filaments…').press('Escape');
	await expect(page.getByPlaceholder('Search filaments…')).toBeHidden();

	await page.getByLabel('Lot code').fill(unique('LOT'));
	await page.getByRole('button', { name: 'Save' }).click();
	await expect(page).toHaveURL(/\/spools$/);
});

// spec: 007/FR-041, 007/FR-051
test('spool list: brand / status filters and sort live in the URL and survive a reload', async ({ page, request }) => {
	const brand = unique('Brand');
	const color = unique('Plum');
	await addColor(request, color, '#DDA0DD');
	const filament = await addFilament(request, { brand, material: 'PLA', color });
	await addSpool(request, filament, { lotCode: 'sealed', purchasedAt: '2026-01-10' });
	await addSpool(request, filament, { lotCode: 'in-use', purchasedAt: '2026-02-10', openedAt: '2026-02-11' });

	await page.goto('/spools');
	await choose(page, 'All brands', brand);
	await expect(page).toHaveURL(new RegExp(`brand=${encodeURIComponent(brand).replace(/%20/g, '\\+')}`));
	const rows = page.getByRole('row').filter({ hasText: brand });
	await expect(rows).toHaveCount(2);
	await expect(rows.first().locator('span[title="#DDA0DD"]')).toBeVisible();

	await choose(page, 'Any status', 'Opened');
	await expect(rows).toHaveCount(1);
	await expect(rows).toContainText('in-use');
	await page.reload();
	await expect(page.getByRole('row').filter({ hasText: brand })).toHaveCount(1);

	await page.goto(`/spools?brand=${encodeURIComponent(brand)}`);
	await page.getByRole('button', { name: /^Purchased/ }).click();
	await page.getByRole('button', { name: /^Purchased/ }).click();
	await expect(page).toHaveURL(/sort=purchasedAt&order=desc|order=desc.*sort=purchasedAt/);
	await expect(page.getByRole('row').filter({ hasText: brand }).first()).toContainText('in-use');
});

// spec: 007/FR-041, 007/FR-033
test('spool list: deleting a spool a profile uses shows why it was refused', async ({ page, request }) => {
	const brand = unique('Brand');
	const filament = await addFilament(request, { brand, material: 'PLA', color: 'Black' });
	const spool = await addSpool(request, filament, { lotCode: unique('LOT') });
	await addProfile(request, filament, unique('Profile'), '220', spool);

	await page.goto(`/spools?brand=${encodeURIComponent(brand)}`);
	await page.getByRole('row').filter({ hasText: brand }).getByRole('button', { name: 'Delete' }).click();
	await expect(page.getByText("Can't delete — a Print Profile references this spool.")).toBeVisible();
});
