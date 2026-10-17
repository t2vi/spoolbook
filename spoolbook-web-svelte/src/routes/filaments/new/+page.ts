import { redirect } from '@sveltejs/kit';

// Add forms are modals on the list now (002/FR-015); old links land there.
export function load() {
	redirect(307, '/filaments?new');
}
