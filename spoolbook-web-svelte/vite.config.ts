import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) => filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// Every page here fetches client-side (see every route's $effect-based data loading) —
			// nothing depends on SvelteKit's SSR data loading, so a static SPA build + fallback.html
			// is enough. spoolbook-rs serves the build/ output directly (main.rs) — no Node process
			// needed. ssr disabled in src/routes/+layout.ts to match.
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		// Dev-time proxy to spoolbook-rs (cargo run, port 5070) — keeps every request same-origin
		// from the browser's perspective so the shared cookie auth just works, no CORS needed.
		// /login and /logout are SvelteKit's own routes (see routes/login, +layout.svelte's
		// signOut) — only /api and the camera stream are real backend routes.
		proxy: {
			'/api': 'http://localhost:5070',
			'^/printers/\\d+/camera': 'http://localhost:5070'
		}
	}
});
