import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-auto';
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

			// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
			// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
			// See https://svelte.dev/docs/kit/adapters for more information about adapters.
			adapter: adapter()
		})
	],
	server: {
		// Dev-time proxy to Spoolbook.Web (dotnet run, port 5070) — keeps every request
		// same-origin from the browser's perspective so the shared cookie auth just works,
		// no CORS needed during the migration. Only /printers/{id}/camera proxies through
		// (regex-keyed) — plain /printers/* is SvelteKit's own routing once that page is ported.
		proxy: {
			'/api': 'http://localhost:5070',
			'/login': 'http://localhost:5070',
			'/logout': 'http://localhost:5070',
			'^/printers/\\d+/camera': 'http://localhost:5070'
		}
	}
});
