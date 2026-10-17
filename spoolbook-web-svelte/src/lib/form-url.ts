// A list page's open add/edit form lives in its URL (002/FR-015): `?new` → null (add),
// `?edit=<id>` → that id, neither → undefined (closed). Everything else in the query (filters,
// sort, page) is left alone.
export function openForm(url: URL): number | null | undefined {
	if (url.searchParams.has('new')) return null;
	const id = Number(url.searchParams.get('edit'));
	return id > 0 ? id : undefined;
}

export function formHref(url: URL, form: number | null | undefined): string {
	const params = listParams(url);
	if (form === null) params.set('new', '');
	else if (form !== undefined) params.set('edit', String(form));
	const query = params.toString().replace(/(^|&)new=(?=&|$)/, '$1new');
	return `${url.pathname}${query ? `?${query}` : ''}`;
}

// The query without the form keys — what a list fetch should send.
export function listParams(url: URL): URLSearchParams {
	const params = new URLSearchParams(url.searchParams);
	params.delete('new');
	params.delete('edit');
	return params;
}
