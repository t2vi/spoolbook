import { expect, test } from '@playwright/test';
import { addFilament, addSpool, unique } from './helpers';

// spec: 002/FR-008
test('leaving an edit form with unsaved changes asks first; saving clears it', async ({ page, request }) => {
	const spool = await addSpool(request, await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' }), { lotCode: 'before' });
	await page.goto(`/spools/edit/${spool}`);
	await expect(page.getByLabel('Lot code')).toHaveValue('before');

	// One handler for the whole test, answering from a queue: a dialog nobody expects finds the
	// queue empty, gets dismissed (blocking the navigation) and is recorded.
	const answers: boolean[] = [];
	const unexpected: string[] = [];
	page.on('dialog', (d) => {
		const answer = answers.shift();
		if (answer === undefined) unexpected.push(d.message());
		return answer ? d.accept() : d.dismiss();
	});

	// Untouched: leaving doesn't ask.
	await page.getByRole('link', { name: 'Colors' }).click();
	await expect(page).toHaveURL(/\/filaments\/colors$/);

	await page.goto(`/spools/edit/${spool}`);
	await expect(page.getByLabel('Lot code')).toHaveValue('before');
	await page.getByLabel('Lot code').fill('after');
	answers.push(false);
	await page.getByRole('link', { name: 'Colors' }).click();
	await expect(page).toHaveURL(new RegExp(`/spools/edit/${spool}$`));
	await expect(page.getByLabel('Lot code')).toHaveValue('after');

	answers.push(true);
	await page.getByRole('link', { name: 'Colors' }).click();
	await expect(page).toHaveURL(/\/filaments\/colors$/);

	await page.goto(`/spools/edit/${spool}`);
	await page.getByLabel('Lot code').fill('saved');
	await page.getByRole('button', { name: 'Save' }).click();
	await expect(page).toHaveURL(/\/spools$/);
	expect(unexpected).toEqual([]);
	expect(answers).toEqual([]);
});

// spec: 002/FR-008
test('closing the tab with unsaved changes triggers the browser prompt', async ({ page, request }) => {
	const spool = await addSpool(request, await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' }));
	await page.goto(`/spools/edit/${spool}`);
	await page.getByLabel('Lot code').fill('unsaved');
	const prompt = page.waitForEvent('dialog');
	await page.close({ runBeforeUnload: true });
	expect((await prompt).type()).toBe('beforeunload');
});
