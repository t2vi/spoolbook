import { expect, test } from '@playwright/test';
import { addFilament, unique } from './helpers';

// spec: 002/FR-009
test('top bar: the breadcrumb shows home, group and page, with New/Edit/Details on sub-pages', async ({ page, request }) => {
	const crumbs = page.getByRole('navigation', { name: 'breadcrumb' });

	await page.goto('/filaments/colors');
	await expect(crumbs).toHaveText(/Filaments\s*Colors/);
	await expect(crumbs.getByRole('link', { name: 'Home' })).toHaveAttribute('href', '/');
	// The current page is marked aria-current, not an <a>.
	await expect(crumbs.locator('a', { hasText: 'Colors' })).toHaveCount(0);
	await expect(crumbs.getByText('Colors')).toHaveAttribute('aria-current', 'page');

	const id = await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' });
	await page.goto(`/filaments/edit/${id}`);
	await expect(crumbs).toHaveText(/Filaments\s*List\s*Edit/);
	await expect(crumbs.getByRole('link', { name: 'List' })).toHaveAttribute('href', '/filaments');

	await page.goto('/');
	await expect(crumbs).toHaveText(/Dashboard/);
});

// spec: 002/FR-009
test('top bar: the theme follows the system first, then the switch, remembered across reloads', async ({ page }) => {
	await page.emulateMedia({ colorScheme: 'dark' });
	await page.goto('/');
	const html = page.locator('html');
	await expect(html).toHaveClass(/\bdark\b/);

	await page.getByRole('button', { name: 'Switch to light mode' }).click();
	await expect(html).not.toHaveClass(/\bdark\b/);
	await page.reload();
	await expect(html).not.toHaveClass(/\bdark\b/);

	await page.getByRole('button', { name: 'Switch to dark mode' }).click();
	await page.emulateMedia({ colorScheme: 'light' });
	await page.reload();
	await expect(html).toHaveClass(/\bdark\b/);
});
