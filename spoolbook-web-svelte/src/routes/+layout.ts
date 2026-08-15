// No page here uses SvelteKit's SSR data loading — every route fetches client-side. Disabling
// SSR makes that explicit and is required for adapter-static's fallback (SPA) mode.
export const ssr = false;
