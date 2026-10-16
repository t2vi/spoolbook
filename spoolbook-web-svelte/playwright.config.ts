import { defineConfig } from '@playwright/test';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// e2e runs the real backend (spoolbook-rs) serving this app's static build, against a throwaway
// DB. The config is evaluated again in every worker, so the data dir is fixed through the env
// on first evaluation and inherited after.
process.env.E2E_DATA_DIR ??= mkdtempSync(join(tmpdir(), 'spoolbook-e2e-'));
export const DB_PATH = join(process.env.E2E_DATA_DIR, 'e2e.db');
export const AUTH_STATE = join(process.env.E2E_DATA_DIR, 'auth.json');

export default defineConfig({
	testDir: 'e2e',
	// One backend and one DB: tests run in order and name their own data uniquely.
	workers: 1,
	reporter: process.env.CI ? 'list' : 'line',
	use: { baseURL: 'http://127.0.0.1:5070', trace: 'retain-on-failure' },
	projects: [
		{ name: 'setup', testMatch: /setup\.e2e\.ts/ },
		{ name: 'signed-out', testMatch: /signed-out\.e2e\.ts/, dependencies: ['setup'] },
		{ name: 'app', testMatch: /\.e2e\.ts/, testIgnore: /(setup|signed-out)\.e2e\.ts/, dependencies: ['setup'], use: { storageState: AUTH_STATE } }
	],
	webServer: {
		command: 'cargo run --quiet --manifest-path ../spoolbook-rs/Cargo.toml',
		url: 'http://127.0.0.1:5070/api/version',
		// Never test against whatever is already on :5070 (likely your dev DB).
		reuseExistingServer: false,
		timeout: 600_000,
		env: {
			SPOOLBOOK_DB_PATH: DB_PATH,
			SPOOLBOOK_STATIC_ROOT: fileURLToPath(new URL('./build', import.meta.url)),
			// Startup catalog sync fails fast instead of importing the real 1,600-row feed.
			FILAMENT_CATALOG_URL: 'http://127.0.0.1:9/filament-catalog.json'
		}
	}
});
