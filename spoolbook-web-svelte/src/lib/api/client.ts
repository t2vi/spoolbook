// Thin fetch wrappers over spoolbook-rs's JSON API. Relative paths only — same-origin in prod,
// proxied to the backend by vite.config.ts's dev server so cookie auth just works with no CORS
// setup.
import type {
	ApiResult,
	DashboardSnapshot,
	FailureMode,
	Filament,
	FilamentColor,
	FilamentSearchResult,
	ImportResult,
	PrinterControlResult,
	PrinterJob,
	PrinterLiveSnapshot,
	Print,
	PrintInventoryResult,
	PrintProfile,
	PrintStatus,
	Printer,
	Project,
	ProjectPlate,
	ProjectResult,
	ProfileFieldSpecResponse,
	ProfileInventoryResult,
	ProfileSource,
	ReslicingResult,
	SlicerType,
	Spool
} from './types';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(path, {
		credentials: 'include',
		headers: init?.body && !(init.body instanceof FormData) ? { 'Content-Type': 'application/json' } : undefined,
		...init
	});
	const body = await res.json().catch(() => null);
	if (!res.ok) throw new Error(body?.error ?? `${path} failed (${res.status})`);
	return body as T;
}

const json = (body: unknown): RequestInit => ({ method: 'POST', body: JSON.stringify(body) });

// ProfileResult is the one Result type with field-level Errors (a dictionary, not a single
// Error string) — the caller needs the parsed body even on a 400 to show them, so this variant
// doesn't throw on !ok the way request() does.
async function requestAllowingError<T>(path: string, init: RequestInit): Promise<T> {
	const res = await fetch(path, {
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		...init
	});
	return (await res.json()) as T;
}

export const me = () => request<{ authenticated: boolean; googleLinked?: boolean }>('/api/me');
export const setupStatus = () => request<{ needsSetup: boolean }>('/api/setup-status');
// requestAllowingError, not request: a wrong password / already-set-up is a normal error response
// the caller needs to show inline, not throw-and-catch.
export const setup = (username: string, password: string) =>
	requestAllowingError<ApiResult>('/api/setup', json({ username, password }));
export const login = (username: string, password: string) =>
	requestAllowingError<ApiResult>('/api/login', json({ username, password }));
export const logout = () => request<ApiResult>('/api/logout', { method: 'POST' });
export const updateAccount = (currentPassword: string, newUsername?: string, newPassword?: string) =>
	requestAllowingError<ApiResult>('/api/account', {
		method: 'PUT',
		body: JSON.stringify({ currentPassword, newUsername, newPassword })
	});
export const getDashboard = () => request<DashboardSnapshot>('/api/dashboard');

// Google OAuth is link-only for this release: an already-authenticated admin links their
// account from Settings, and /login's "Sign in with Google" only works if that link already
// exists. Login/link itself is a plain browser navigation (redirects to Google, then back to
// the callback), not a fetch -- callers just set window.location.href to '/api/auth/google/login'.
export const googleStatus = () => request<{ configured: boolean }>('/api/auth/google/status');

export interface GoogleConfig {
	clientId: string | null;
	redirectUri: string | null;
	secretSet: boolean;
}

export const getGoogleConfig = () => request<GoogleConfig>('/api/auth/google/config');
// Blank clientSecret means "keep the existing one" -- the GET endpoint never returns it.
export const saveGoogleConfig = (clientId: string, clientSecret: string, redirectUri: string) =>
	requestAllowingError<ApiResult>('/api/auth/google/config', {
		method: 'PUT',
		body: JSON.stringify({ clientId, clientSecret, redirectUri })
	});
export const unlinkGoogle = () => request<ApiResult>('/api/auth/google', { method: 'DELETE' });

export interface SettingsResponse {
	additionalFilamentSourceUrls: string | null;
	lastFilamentSyncAt: string | null;
	catalogUrl: string;
	latitude: number | null;
	longitude: number | null;
}

export const getSettings = () => request<SettingsResponse>('/api/settings');
export const saveSettings = (additionalFilamentSourceUrls: string | null, latitude: number | null, longitude: number | null) =>
	request<{ ok: boolean }>('/api/settings', json({ additionalFilamentSourceUrls, latitude, longitude }));

export interface PrinterInput {
	name: string;
	model: string | null;
	ipAddress: string | null;
	accessCode: string | null;
	serialNumber: string | null;
}

export interface PrinterResult extends PrinterControlResult {
	printer: Printer | null;
}

export const listPrinters = () => request<Printer[]>('/api/printers');
// No GET /api/printers/{id} endpoint — PrinterService itself has no single-item lookup, only
// ListAsync(); the Blazor edit page finds-by-id client-side from the full list, same here.
export const getPrinter = async (id: number) => (await listPrinters()).find((p) => p.id === id) ?? null;
export const createPrinter = (input: PrinterInput) => request<PrinterResult>('/api/printers', json(input));
export const updatePrinter = (id: number, input: PrinterInput) =>
	request<PrinterResult>(`/api/printers/${id}`, { method: 'PUT', body: JSON.stringify(input) });
export const deletePrinter = (id: number) => request<PrinterControlResult>(`/api/printers/${id}`, { method: 'DELETE' });
export const testPrinterConnection = (ipAddress: string, accessCode: string) =>
	request<PrinterControlResult>('/api/printers/test', json({ ipAddress, accessCode }));

export const controlPrinter = (id: number, command: 'pause' | 'resume' | 'stop') =>
	request<PrinterControlResult>(`/api/printers/${id}/control`, json({ command }));

export const retryPrinterCamera = (id: number) =>
	request<PrinterControlResult>(`/api/printers/${id}/camera/retry`, { method: 'POST' });

export interface StartPrintRequest {
	projectId: number;
	platerId: string;
	spoolId: number;
	profileId: number;
	useAms: boolean;
	amsSlot: number;
}

export const startPrint = (printerId: number, req: StartPrintRequest) =>
	request<PrinterControlResult>(`/api/printers/${printerId}/print`, json(req));

// Native EventSource — reconnection/framing handled by the browser, no client library needed.
export function subscribeToPrinterLiveStatus(
	printerId: number,
	onSnapshot: (snapshot: PrinterLiveSnapshot) => void
): () => void {
	const source = new EventSource(`/api/printers/${printerId}/live`);
	source.onmessage = (e) => onSnapshot(JSON.parse(e.data));
	return () => source.close();
}

export const listProjects = () => request<Project[]>('/api/projects');
export const getProjectPlates = (projectId: number) => request<ProjectPlate[]>(`/api/projects/${projectId}/plates`);

export async function uploadProject(file: File): Promise<ProjectResult> {
	const form = new FormData();
	form.append('file', file);
	return request<ProjectResult>('/api/projects/upload', { method: 'POST', body: form });
}

export const importProjectFromUrl = (url: string) => request<ProjectResult>('/api/projects/import-url', json({ url }));

export const renameProject = (projectId: number, fileName: string) =>
	request<ProjectResult>(`/api/projects/${projectId}`, { method: 'PUT', body: JSON.stringify({ fileName }) });
export const deleteProject = (projectId: number) => request<{ ok: boolean; error?: string }>(`/api/projects/${projectId}`, { method: 'DELETE' });

export const resliceProject = (projectId: number, profileId: number) =>
	request<ReslicingResult>(`/api/projects/${projectId}/reslice`, json({ profileId }));

export const listRecentPrints = (printerId: number) => request<Print[]>(`/api/prints?printerId=${printerId}`);
export const recommendProfile = (projectId: number) =>
	request<PrintProfile | null>(`/api/prints/recommend-profile?projectId=${projectId}`);

export const searchPrints = (status: PrintStatus | '', printerId: number | null, page: number, pageSize = 20) => {
	// int?/enum? query params fail to bind from an empty string (unlike string?, which just
	// gives ""), so omit them entirely rather than sending status= / printerId= blank.
	const params = new URLSearchParams({ page: String(page), pageSize: String(pageSize) });
	if (status) params.set('status', status);
	if (printerId !== null) params.set('printerId', String(printerId));
	return request<PrintInventoryResult>(`/api/prints/inventory?${params}`);
};
export const getPrint = (id: number) => request<Print>(`/api/prints/${id}`);
export const deletePrint = (id: number) => request<ApiResult>(`/api/prints/${id}`, { method: 'DELETE' });
export const findJobMatch = (printerId: number, startedAt: string) =>
	request<PrinterJob | null>(`/api/prints/job-match?printerId=${printerId}&startedAt=${encodeURIComponent(startedAt)}`);
export const attachJobToPrint = (printId: number, jobId: number) =>
	request<{ ok: boolean }>(`/api/prints/${printId}/attach-job`, json({ jobId }));

export interface PrintInput {
	startedAt: string;
	endedAt: string;
	status: PrintStatus;
	notes: string | null;
	amsHumidityPct: number | null;
	actualRoomTempC: number | null;
	cleanBuildPlate: boolean | null;
	projectId: number | null;
	projectPlaterId: string | null;
	failureModes: FailureMode[];
}

export interface PrintResult extends ApiResult {
	print: Print | null;
}

export const createPrint = (profileId: number, spoolId: number, printerId: number, input: PrintInput) =>
	request<PrintResult>('/api/prints', json({ profileId, spoolId, printerId, input }));
export const updatePrint = (id: number, printerId: number, input: PrintInput) =>
	request<PrintResult>(`/api/prints/${id}`, { method: 'PUT', body: JSON.stringify({ printerId, input }) });

export const findVersionCandidate = (meshHash: string | null, fileName: string, excludeProjectId: number) =>
	request<Project | null>(
		`/api/projects/version-candidate?meshHash=${meshHash ?? ''}&fileName=${encodeURIComponent(fileName)}&excludeProjectId=${excludeProjectId}`
	);
export const linkProjectVersion = (projectId: number, previousVersionProjectId: number) =>
	request<{ ok: boolean }>(`/api/projects/${projectId}/link-version`, json({ previousVersionProjectId }));

export const listSpools = () => request<Spool[]>('/api/spools');
export const getSpool = (id: number) => request<Spool>(`/api/spools/${id}`);
export const listProfilesForFilament = (filamentId: number) =>
	request<PrintProfile[]>(`/api/profiles?filamentId=${filamentId}`);
export const searchProfiles = () => request<ProfileInventoryResult>('/api/profiles/inventory');
export const getProfileFieldSpec = (profileId: number | null) =>
	request<ProfileFieldSpecResponse>(`/api/profiles/field-spec${profileId === null ? '' : `?profileId=${profileId}`}`);
export const deleteProfile = (id: number) => request<ApiResult>(`/api/profiles/${id}`, { method: 'DELETE' });

// Accepts a sliced .3mf or a raw Bambu Studio preset export (.json) -- format is auto-detected
// server-side (github.com/t2vi/spoolbook/issues/99).
export async function importProfilePreset(file: File): Promise<ImportResult> {
	const form = new FormData();
	form.append('file', file);
	return request<ImportResult>('/api/profiles/import-preset', { method: 'POST', body: form });
}

// Bambu Studio's bundled system filament preset library (via slicer-service, see bambu_import.rs).
export const listSystemPresets = () => request<{ ok: boolean; names: string[] }>('/api/profiles/system-presets');
export const resolveSystemPreset = (name: string) =>
	request<ImportResult>('/api/profiles/system-presets/resolve', json({ name }));

export interface ProfileInput {
	name: string;
	fields: Record<string, string>;
	source: ProfileSource | null;
	sourceSlicer: SlicerType | null;
	rawSettingsJson: string | null;
	spoolId: number | null;
}

export interface ProfileResult extends ApiResult {
	profile: PrintProfile | null;
	errors?: Record<string, string> | null;
}

export const createProfile = (filamentId: number, input: ProfileInput) =>
	requestAllowingError<ProfileResult>(`/api/profiles?filamentId=${filamentId}`, json(input));
export const updateProfile = (id: number, input: ProfileInput) =>
	requestAllowingError<ProfileResult>(`/api/profiles/${id}`, { method: 'PUT', body: JSON.stringify(input) });

export interface SpoolInput {
	lotCode: string | null;
	purchasedAt: string | null;
	openedAt: string | null;
	emptiedAt: string | null;
	weightGrams: number | null;
	diameterMm: number | null;
	notes: string | null;
}

export interface SpoolResult extends ApiResult {
	spool: Spool | null;
}

export const createSpool = (filamentId: number, input: SpoolInput) =>
	request<SpoolResult>('/api/spools', json({ filamentId, ...input }));
export const updateSpool = (id: number, input: SpoolInput) =>
	request<SpoolResult>(`/api/spools/${id}`, { method: 'PUT', body: JSON.stringify(input) });
export const deleteSpool = (id: number) => request<SpoolResult>(`/api/spools/${id}`, { method: 'DELETE' });

export interface FilamentInput {
	brand: string;
	material: string;
	variant: string | null;
	color: string;
}

export interface FilamentResult extends ApiResult {
	entry: Filament | null;
}

export const searchFilaments = (brand: string, material: string, page: number, pageSize = 20) =>
	request<FilamentSearchResult>(
		`/api/filaments?brand=${encodeURIComponent(brand)}&material=${encodeURIComponent(material)}&page=${page}&pageSize=${pageSize}`
	);
export const listAllFilaments = () => request<Filament[]>('/api/filaments/all');
export const createFilament = (input: FilamentInput) => request<FilamentResult>('/api/filaments', json(input));
export const updateFilament = (id: number, input: FilamentInput) =>
	request<FilamentResult>(`/api/filaments/${id}`, { method: 'PUT', body: JSON.stringify(input) });
export const deleteFilament = (id: number) => request<FilamentResult>(`/api/filaments/${id}`, { method: 'DELETE' });
export const syncFilamentCatalog = () =>
	request<{ ok: boolean; error?: string; added?: number; skipped?: number }>('/api/filaments/sync', { method: 'POST' });
export const listFilamentColors = () => request<FilamentColor[]>('/api/filament-colors');

// Bambu addresses AMS slots across units as unit_id * 4 + slot_id — same convention PrintModal.razor uses.
export function amsSlotNumber(tray: { unitId: string; slotId: string }): number {
	return parseInt(tray.unitId, 10) * 4 + parseInt(tray.slotId, 10);
}
