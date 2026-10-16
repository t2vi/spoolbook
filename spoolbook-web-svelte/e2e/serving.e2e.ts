import { expect, test } from '@playwright/test';
import { existsSync } from 'node:fs';
import { DB_PATH } from '../playwright.config';

// spec: 002/FR-001
test('the DB is created at SPOOLBOOK_DB_PATH', () => {
	expect(existsSync(DB_PATH)).toBe(true);
});

// spec: 002/FR-003, 002/FR-004
test('a deep link to a client-side route returns the app with 200', async ({ request }) => {
	const res = await request.get('/spools/edit/12345');
	expect(res.status()).toBe(200);
	expect(await res.text()).toContain('<html');
});

// spec: 002/FR-002, 002/FR-007
test('the app is up after startup and shows its version and Source link', async ({ page, request }) => {
	const { version } = await (await request.get('/api/version')).json();
	await page.goto('/');
	await expect(page.getByText(`v${version}`).first()).toBeVisible();
	await expect(page.getByRole('link', { name: 'Source' }).first()).toHaveAttribute('href', 'https://github.com/t2vi/spoolbook');
});
