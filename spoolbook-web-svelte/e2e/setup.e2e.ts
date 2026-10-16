import { expect, test } from '@playwright/test';
import { AUTH_STATE } from '../playwright.config';
import { USER } from './helpers';

// spec: 008/FR-040, 008/FR-041
test('a fresh install sends every page to setup, which creates the first account', async ({ page }) => {
	await page.goto('/spools');
	await expect(page).toHaveURL(/\/setup$/);
	await page.getByPlaceholder('Username').fill(USER.username);
	await page.getByPlaceholder('Password (min. 8 characters)').fill(USER.password);
	await page.getByPlaceholder('Confirm password').fill(USER.password);
	await page.getByRole('button', { name: 'Create account' }).click();
	await expect(page).not.toHaveURL(/\/setup$/);
	await page.context().storageState({ path: AUTH_STATE });
});
