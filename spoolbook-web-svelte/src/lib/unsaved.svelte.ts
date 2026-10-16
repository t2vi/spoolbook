import { beforeNavigate } from '$app/navigation';
import { untrack } from 'svelte';

// Unsaved-changes guard (002/FR-008). Call once during component init with a snapshot of each
// form's fields (a page with several independently saved forms names each one). A part starts
// being tracked when it's marked clean — after its values load, or at once for a blank form —
// and is dirty while its snapshot differs from that. Leaving with anything dirty asks first: an
// in-app link or back gets a confirm(); reload or closing the tab gets the browser's own prompt.
// Saving or cancelling marks it clean again.
export function unsavedGuard<K extends string>(parts: Record<K, () => unknown>) {
	const keys = Object.keys(parts) as K[];
	const baselines = $state<Partial<Record<K, string>>>({});
	const isDirty = (key: K) => baselines[key] !== undefined && JSON.stringify(parts[key]()) !== baselines[key];

	beforeNavigate((navigation) => {
		if (!keys.some(isDirty)) return;
		// 'leave' = reload / close tab / external URL: cancelling shows the browser's own dialog.
		if (navigation.type === 'leave' || !confirm('You have unsaved changes. Leave without saving?')) navigation.cancel();
	});

	return {
		// Untracked: called from an $effect, reading the fields would re-run it on every edit.
		markClean(...only: K[]) {
			untrack(() => {
				for (const key of only.length > 0 ? only : keys) baselines[key] = JSON.stringify(parts[key]());
			});
		},
		get dirty() {
			return keys.some(isDirty);
		}
	};
}
