import { expect, test } from '@playwright/test';
import { addFilament, addPrinter, addProfile, addSpool, attachProject, seedPrint, seedProject, unique } from './helpers';

// spec: 005/FR-040, 005/FR-041
test('projects: rename, open the detail page, and deleting a printed project is refused with its name', async ({ page, request }) => {
	const filament = await addFilament(request, { brand: unique('Brand'), material: 'PLA', color: 'Black' });
	const spool = await addSpool(request, filament);
	const profile = await addProfile(request, filament, unique('Profile'));
	const name = `${unique('benchy').replace(' ', '-')}.3mf`;
	const project = seedProject(name);
	attachProject(seedPrint({ profileId: profile, spoolId: spool, printerId: await addPrinter(request, unique('Printer')), status: 'Success', startedAt: '2026-10-05T10:00:00Z' }), project);

	await page.goto('/projects');
	const row = page.getByRole('row').filter({ hasText: name });
	await expect(row).toHaveCount(1);
	await row.getByRole('link', { name }).click();
	await expect(page).toHaveURL(new RegExp(`/projects/${project}$`));
	await expect(page.getByText(/v1/).first()).toBeVisible();

	await page.goto('/projects');
	const renamed = `renamed-${name}`;
	await row.getByRole('button', { name: 'Rename' }).click();
	await page.getByRole('dialog').getByLabel('Name').fill(renamed);
	await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();
	const renamedRow = page.getByRole('row').filter({ hasText: renamed });
	await expect(renamedRow).toHaveCount(1);

	await renamedRow.getByRole('button', { name: 'Delete' }).click();
	await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();
	await expect(page.getByRole('alertdialog')).toContainText("Can't delete");
	await expect(page.getByRole('alertdialog')).toContainText(renamed);
});
