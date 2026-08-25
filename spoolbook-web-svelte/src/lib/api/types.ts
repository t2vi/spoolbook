// Mirrors the JSON shapes returned by spoolbook-rs's API — camelCase (serde's rename_all), enums
// as their Rust variant name string. These interfaces only declare the fields actually used here
// rather than every column the backend happens to return — TS doesn't object-literal-check a
// fetch() response, so extra runtime fields are harmless.

export interface Printer {
	id: number;
	name: string;
	model: string | null;
	ipAddress: string | null;
	accessCode: string | null;
	serialNumber: string | null;
}

export interface AmsTrayReading {
	slotId: string;
	materialType: string | null;
	colorHex: string | null;
	remainPercent: number | null;
}

export interface AmsUnitReading {
	unitId: string;
	humidityLevel: number | null;
	trays: AmsTrayReading[];
}

export type CameraStatus = 'NotStarted' | 'Connecting' | 'Streaming' | 'Unavailable';

export interface PrinterLiveSnapshot {
	connected: boolean;
	amsUnits: AmsUnitReading[];
	cameraStatus: CameraStatus;
	cameraError: string | null;
	gcodeState: string | null;
}

export type PrintStatus = 'Success' | 'Failed' | 'Partial' | 'InProgress';

export interface Filament {
	id: number;
	brand: string;
	material: string;
	variant: string | null;
	color: string;
}

export interface Spool {
	id: number;
	filamentId: number;
	filament: Filament | null;
	lotCode: string | null;
	// DateOnly serializes as "yyyy-MM-dd" — matches <input type="date"> value format directly.
	purchasedAt: string | null;
	openedAt: string | null;
	emptiedAt: string | null;
	weightGrams: number | null;
	diameterMm: number | null;
	notes: string | null;
}

export interface FilamentColor {
	id: number;
	name: string;
	hex: string;
}

export interface FilamentSearchResult {
	entries: Filament[];
	total: number;
	page: number;
	pageSize: number;
	totalPages: number;
}

export interface PrintProfile {
	id: number;
	name: string;
	// The entity carries ~140 slicer-setting fields beyond these — only what the list/nav views
	// actually read is declared here (fetch responses aren't object-literal-checked by TS).
	filamentId?: number;
	filament?: Filament | null;
	nozzleTempC?: number | null;
	hotPlateTempC?: number | null;
	printSpeedMmS?: number | null;
	source?: string;
}

export interface ProfileFieldEntry {
	name: string;
	label: string;
	unit: string;
	isBool: boolean;
	isTextArea: boolean;
	isNumeric: boolean;
	options: string[] | null;
	isEnum: boolean;
	isPlainText: boolean;
	hideWhenBlank: boolean;
	showRow: boolean;
	boolValue: boolean;
	value: string;
}

export interface ProfileFieldGroup {
	title: string;
	fields: ProfileFieldEntry[];
}

export interface ProfileFieldTab {
	title: string;
	sections: ProfileFieldGroup[];
}

export interface ProfileFieldSpecResponse {
	name: string;
	tabs: ProfileFieldTab[];
}

export interface ProfileInventoryResult {
	profiles: PrintProfile[];
	total: number;
	page: number;
	pageSize: number;
	totalPages: number;
}

export type ProfileSource = 'Manual' | 'SlicerImport';
export type SlicerType = 'PrusaSlicer' | 'OrcaSlicer' | 'BambuStudio';

export interface ImportResult {
	ok: boolean;
	error?: string | null;
	suggestedName: string | null;
	fields: Record<string, string> | null;
	rawSettingsJson: string | null;
}

export type FailureMode = 'Stringing' | 'LayerAdhesion' | 'Warping' | 'UnderExtrusion' | 'OverExtrusion' | 'LayerShift' | 'Clog' | 'Other';

export interface PrintFailureModeEntry {
	mode: FailureMode;
}

export interface Print {
	id: number;
	profileId: number;
	profile: PrintProfile | null;
	printerId: number;
	printer: { id: number; name: string } | null;
	spoolId: number;
	spool: Spool | null;
	projectId: number | null;
	project: Project | null;
	projectPlaterId: string | null;
	startedAt: string;
	endedAt: string | null;
	status: PrintStatus;
	notes: string | null;
	amsHumidityPct: number | null;
	actualRoomTempC: number | null;
	cleanBuildPlate: boolean | null;
	// Auto-fetched from Open-Meteo when the print ends (issues/94) -- never manually entered.
	ambientTempC: number | null;
	ambientHumidityPct: number | null;
	ambientSource: string | null;
	failureModes: PrintFailureModeEntry[];
}

export interface PrintInventoryResult {
	prints: Print[];
	total: number;
	page: number;
	pageSize: number;
	totalPages: number;
}

export interface PrinterJob {
	id: number;
	printerId: number;
	externalJobId: string;
	startedAt: string;
	endedAt: string | null;
	printId: number | null;
}

export interface Project {
	id: number;
	fileName: string;
	filePath: string;
	lastKnownWriteTimeUtc: string;
	lastKnownFileSizeBytes: number;
	meshHash: string | null;
	previousVersionProjectId: number | null;
	versionNumber: number;
	isCurrentVersion: boolean;
}

export interface ProjectPlate {
	platerId: string;
	platerName: string | null;
	thumbnailBytes: string | null; // base64
}

export interface ApiResult {
	ok: boolean;
	error?: string | null;
}

export interface PrinterControlResult extends ApiResult {}

export interface ProjectResult extends ApiResult {
	project: Project | null;
	created: boolean;
}

export interface ReslicingResult extends ApiResult {
	project: Project | null;
}

export interface CategoryCount {
	label: string;
	count: number;
}

export interface DashboardMetrics {
	filamentCount: number;
	lastFilamentSyncAt: string | null;
	filamentsByBrand: CategoryCount[];
	filamentsByMaterial: CategoryCount[];
	spoolsByStatus: CategoryCount[];
	printsByStatus: CategoryCount[];
}

export interface DashboardSnapshot {
	metrics: DashboardMetrics;
	profileCount: number;
}
