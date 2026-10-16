import { expect, test } from '@playwright/test';
import { USER } from './helpers';

// spec: 008/FR-041, 002/FR-007
test('signed out, every page shows only the sign-in form, the version and the Source link', async ({ page }) => {
	for (const path of ['/', '/filaments', '/spools/new', '/prints', '/settings']) {
		await page.goto(path);
		await expect(page).toHaveURL(/\/login$/);
	}
	await expect(page.getByRole('heading', { name: 'Sign in' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Source' })).toHaveAttribute('href', 'https://github.com/t2vi/spoolbook');
	await expect(page.getByText(/v\d+\.\d+\.\d+/)).toBeVisible();
	await expect(page.getByRole('link', { name: 'Dashboard' })).toHaveCount(0);
	expect((await page.request.get('/api/filaments/all')).status()).toBe(401);
});

// spec: 008/FR-040
test('signing in with username and password lands on the app', async ({ page }) => {
	await page.goto('/login');
	await page.getByPlaceholder('Username').fill(USER.username);
	await page.getByPlaceholder('Password').fill('wrong-password');
	await page.getByRole('button', { name: 'Sign in' }).click();
	await expect(page.getByText(/invalid|wrong|incorrect/i)).toBeVisible();
	await page.getByPlaceholder('Password').fill(USER.password);
	await page.getByRole('button', { name: 'Sign in' }).click();
	await expect(page.getByRole('link', { name: 'Dashboard' })).toBeVisible();
});
