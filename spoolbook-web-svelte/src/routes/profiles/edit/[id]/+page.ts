import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Edit forms are modals on the list now (002/FR-015); old links land there.
export const load: PageLoad = ({ params }) => {
	redirect(307, `/profiles?edit=${params.id}`);
};
