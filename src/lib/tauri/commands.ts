/**
 * The only module that talks to the Rust side. Everything else imports these
 * functions, so the backend can be stubbed in one place for tests.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Player = {
	id: number;
	name: string;
	born: string;
	age: number;
	/** Current Ability, 1-200. Null for staff, who have no attribute block. */
	ability: number | null;
	/** Potential Ability, 1-200. Null for staff. */
	potential: number | null;
	/** True for players; false for staff. */
	is_player: boolean;
	/** The 54 attributes on FM's 1-20 scale. Empty for staff. */
	attributes: number[];
	/** Nation identifier, shared with the club records. */
	nation_id: number;
	/** Nation name where the identifier is confirmed, otherwise empty. */
	nation: string;
	/** Positions the player is comfortable in, strongest first. Empty for staff. */
	positions: string[];
	/** Rating 1-20 for each of the 15 position slots. Empty for staff. */
	position_ratings: number[];
	/** Short name of the club whose squad lists this person; empty when unattached. */
	club: string;
	/** Whether this person is a woman; null when the save can't say. */
	female: boolean | null;
	/** Weekly wage in the save's display currency. Null when out of contract. */
	wage: number | null;
	/** Contract expiry as YYYY-MM-DD; empty when unknown. */
	contract_until: string;
};

export type Club = {
	id: number;
	name: string;
	short_name: string;
	club_id: number;
	nation_id: number;
	/** How many players the squad table lists for this club. */
	squad_size: number;
	/** Mean Current Ability across the squad; null when it has no players. */
	average_ability: number | null;
	/** Mean Potential Ability across the squad. */
	average_potential: number | null;
};

export type SaveSummary = {
	path: string;
	players: Player[];
	clubs: Club[];
	/** Attribute indices that belong to the goalkeeping set. */
	goalkeeping_indices: number[];
	/** Inferred name per attribute index; empty string where not identified. */
	attribute_names: string[];
	/** The 15 position slot names, in slot order. */
	position_names: string[];
	/** The save's in-game date. Null when it could not be read, in which case
	 * ages fall back to the system clock. */
	game_date: string | null;
	frames: number;
	decompressed_bytes: number;
	parse_millis: number;
};

/** Where the file dialogs should open, resolved per platform in Rust. */
export type Locations = {
	/** Football Manager's saves folder, when it exists on this machine. */
	saves: string | null;
	/** The user's Documents folder. */
	documents: string | null;
};

export type ImportResult = {
	matched: string[];
	unmatched: string[];
};

export type Shortlist = {
	name: string;
	players: string[];
};

/** A named filter configuration, saved so a recurring search can be re-run. */
export type SavedFilter = {
	name: string;
	/** The `Filters` object, stored opaquely so the backend never has to know
	 * the shape of a frontend type. */
	filters: unknown;
};

/** How far through parsing the backend is, as it reports it. */
export type ParseProgress = {
	/** 0 to 1. */
	fraction: number;
	/** What the backend is doing, phrased for the user. */
	label: string;
};

export function openSave(path: string): Promise<SaveSummary> {
	const now = new Date();
	// Ages are computed against the user's clock, not the build machine's.
	const today = [now.getFullYear(), now.getMonth() + 1, now.getDate()];
	return invoke<SaveSummary>('open_save', { path, today });
}

/** Subscribes to parse progress. Resolves to the function that stops listening. */
export function onParseProgress(handle: (progress: ParseProgress) => void): Promise<UnlistenFn> {
	return listen<ParseProgress>('parse-progress', (event) => handle(event.payload));
}

export function exportCsv(path: string, rows: Player[]): Promise<void> {
	return invoke('export_csv', { path, rows });
}

/** `known` is every player name in the loaded save, so the backend can report
 * which imported names it could not find. */
export function importCsv(path: string, known: string[]): Promise<ImportResult> {
	return invoke<ImportResult>('import_csv', { path, known });
}

export function defaultLocations(): Promise<Locations> {
	return invoke<Locations>('default_locations');
}

export function loadShortlists(): Promise<Shortlist[]> {
	return invoke<Shortlist[]>('load_shortlists');
}

export function saveShortlists(lists: Shortlist[]): Promise<void> {
	return invoke('save_shortlists', { lists });
}

export function loadFilters(): Promise<SavedFilter[]> {
	return invoke<SavedFilter[]>('load_filters');
}

export function saveFilters(filters: SavedFilter[]): Promise<void> {
	return invoke('save_filters', { filters });
}
