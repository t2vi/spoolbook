import type { APIRequestContext, Page } from '@playwright/test';
import { DatabaseSync } from 'node:sqlite';
import { DB_PATH } from '../playwright.config';

// The account e2e/setup.e2e.ts creates on the fresh DB.
export const USER = { username: 'e2e', password: 'e2e-password' };

// Unique per call, so tests sharing the one DB never collide.
export const unique = (prefix: string) => `${prefix} ${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`;

async function post<T>(api: APIRequestContext, path: string, data: unknown): Promise<T> {
	const res = await api.post(path, { data });
	if (!res.ok()) throw new Error(`${path} → ${res.status()}: ${await res.text()}`);
	return res.json();
}

export async function addFilament(api: APIRequestContext, f: { brand: string; material: string; variant?: string | null; color: string }) {
	return (await post<{ entry: { id: number } }>(api, '/api/filaments', { variant: null, ...f })).entry.id;
}

export async function addColor(api: APIRequestContext, name: string, hex: string) {
	return (await post<{ color: { id: number } }>(api, '/api/filament-colors', { name, hex })).color.id;
}

export async function addSpool(api: APIRequestContext, filamentId: number, fields: Record<string, unknown> = {}) {
	return (await post<{ spool: { id: number } }>(api, '/api/spools', { filamentId, lotCode: null, purchasedAt: null, openedAt: null, emptiedAt: null, weightGrams: null, diameterMm: null, notes: null, ...fields })).spool.id;
}

export async function addProfile(api: APIRequestContext, filamentId: number, name: string, nozzleTemp = '220', spoolId: number | null = null) {
	const body = { name, spoolId, source: 'Manual', sourceSlicer: null, rawSettingsJson: null, fields: { NozzleTempC: nozzleTemp } };
	return (await post<{ profile: { id: number } }>(api, `/api/profiles?filamentId=${filamentId}`, body)).profile.id;
}

export async function addPrinter(api: APIRequestContext, name: string) {
	return (await post<{ printer: { id: number } }>(api, '/api/printers', { name, model: 'P2S', ipAddress: null, accessCode: null, serialNumber: null })).printer.id;
}

// Prints can only be created by sending a job to a printer (003/FR-010), so tests seed them
// straight into the e2e DB.
export function seedPrint(p: { profileId: number; spoolId: number; printerId: number; status: string; startedAt: string; ambientTempC?: number; notes?: string }) {
	const db = new DatabaseSync(DB_PATH);
	try {
		const row = db
			.prepare('INSERT INTO prints (profile_id, spool_id, printer_id, started_at, ended_at, status, notes, ambient_temp_c) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id')
			.get(p.profileId, p.spoolId, p.printerId, p.startedAt, p.status === 'InProgress' ? null : p.startedAt, p.status, p.notes ?? null, p.ambientTempC ?? null) as { id: number };
		return row.id;
	} finally {
		db.close();
	}
}

// Picks `option` from the shadcn Select (Picker) currently showing `current`.
export async function choose(page: Page, current: string | RegExp, option: string) {
	await page.getByRole('button', { name: current, exact: typeof current === 'string' }).click();
	await page.getByRole('option', { name: option, exact: true }).click();
}

// Projects normally come from a .3mf upload; tests that only need the row seed it directly.
export function seedProject(fileName: string) {
	const db = new DatabaseSync(DB_PATH);
	try {
		return (db
			.prepare("INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes) VALUES (?, ?, '2026-01-01T00:00:00Z', 100) RETURNING id")
			.get(`/nonexistent/${fileName}`, fileName) as { id: number }).id;
	} finally {
		db.close();
	}
}

export function attachProject(printId: number, projectId: number) {
	const db = new DatabaseSync(DB_PATH);
	try {
		db.prepare('UPDATE prints SET project_id = ? WHERE id = ?').run(projectId, printId);
	} finally {
		db.close();
	}
}
