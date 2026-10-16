// Type-to-search (007/FR-052): every typed word must appear somewhere in the text, case-insensitive.
export function matchesAllWords(text: string, query: string): boolean {
	const haystack = text.toLowerCase();
	return query.toLowerCase().split(/\s+/).filter(Boolean).every((word) => haystack.includes(word));
}
