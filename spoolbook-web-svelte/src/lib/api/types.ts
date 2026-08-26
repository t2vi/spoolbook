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
	// humidityPct is a real relative-humidity percentage (newer AMS 2 Pro / AMS-HT hardware).
	// Older AMS units have no hygrometer -- humidityLevel (a coarse 1-5 index driving the
	// physical unit's LED ring, not a percentage) is the only signal available for those.
	humidityPct: number | null;
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
	// amsHumidityPct/chamberTempC are auto-snapshotted from the printer's own live MQTT reading
	// when the print ends -- never manually entered, same as the ambient fields below.
	amsHumidityPct: number | null;
	chamberTempC: number | null;
	cleanBuildPlate: boolean | null;
	// Auto-fetched from Open-Meteo when the print ends (issues/94) -- never manually entered.
	ambientTempC: number | null;
	ambientHumidityPct: number | null;
	ambientSource: string | null;
	// Auto-captured at end-of-print regardless of terminal status (issues/121) -- never
	// manually entered either. Null when the printer was unreachable at capture time.
	bedPhotoBase64: string | null;
	failureModes: PrintFailureModeEntry[];
}

export interface HourlyWeatherReading {
	hour: string;
	tempC: number | null;
	humidityPct: number | null;
}

export interface PrintReading {
	recordedAt: string;
	chamberTempC: number | null;
	amsHumidityPct: number | null;
	layerNum: number | null;
	totalLayerNum: number | null;
	progressPct: number | null;
}

// docs/adr/0033 — export/import. Table keys are dynamic (EXPORT_TABLES on the backend), so these
// stay Records rather than a field per table.
export type ImportPreview = Record<string, { total: number; new: number }>;
export type ImportCommitResult = { ok: boolean; tables: Record<string, { inserted: number; matched: number }> };

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
