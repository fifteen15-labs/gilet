/**
 * The only module that talks to the Rust side. Everything else imports these
 * functions, so the backend can be stubbed in one place for tests.
 */
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** A person's non-player sheet. Names come from `staffAttributeNames`, which
 * serves the same 52 for everyone rather than repeating them on every row. */
export type StaffSheet = {
	/** The 52 attributes on FM's 1-20 scale, in the editor's own order. */
	attributes: number[];
	/** Reputation in their home nation, 0-200. */
	homeReputation: number;
	/** Reputation where they currently work, 0-200. */
	currentReputation: number;
	/** Worldwide reputation, 0-200. */
	worldReputation: number;
	/** Non-player Current Ability, 0-200. */
	currentAbility: number;
	/** Non-player Potential Ability, 0-200. */
	potentialAbility: number;
};

export type Player = {
	id: number;
	/** Person entity id — the save's own identifier, used for in-save
	 * shortlist edits. Null for the few unresolved records. */
	eid: number | null;
	name: string;
	born: string;
	/** Age on the save's own date. Null for stubs and compact entries, whose
	 * birth date the save does not carry — an unknown age is not a young one. */
	age: number | null;
	/** Current Ability, 1-200. Null for staff, who have no attribute block. */
	ability: number | null;
	/** Potential Ability, 1-200. Null for staff. */
	potential: number | null;
	/** True for players; false for staff. */
	is_player: boolean;
	/** The 54 attributes on FM's 1-20 scale. Empty for staff. */
	attributes: number[];
	/** Nation identifier, shared with the club records. Null when the save
	 * carries none — stubs and compact entries. */
	nation_id: number | null;
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
	/** Hidden Adaptability, 1-20; null when the personality run is absent. */
	adaptability: number | null;
	/** Hidden Ambition, 1-20. */
	ambition: number | null;
	/** Hidden Loyalty, 1-20. */
	loyalty: number | null;
	/** Hidden Pressure, 1-20. */
	pressure: number | null;
	/** Hidden Professionalism, 1-20 — the development driver. */
	professionalism: number | null;
	/** Hidden Sportsmanship, 1-20. */
	sportsmanship: number | null;
	/** Hidden Temperament, 1-20. */
	temperament: number | null;
	/** Hidden Controversy, 1-20. */
	controversy: number | null;
	/** The pre-game editor's "All Attributes" sheet, read from the entity
	 * object one id below this person's own. Null for anyone with no such
	 * object, which is most players. */
	staff: StaffSheet | null;
	/** True for a stub — a non-contract squad filler the save stores without a
	 * person record. Name, age and attributes are undecoded, not absent. */
	stub: boolean;
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

/** A shortlist as FM stores it inside the save, members already resolved to
 * the same names the player table uses. */
export type GameShortlist = {
	/** The name given in FM; null for the unnamed default list. */
	name: string | null;
	players: string[];
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
	/** The human manager's shortlists read from the save itself. */
	game_shortlists: GameShortlist[];
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

/**
 * A user-defined weighting over attribute indices, used to rank players by
 * what the user is looking for. The weights are the user's: Gilet ships no
 * role table, because FM's own role weights are not published and guessing
 * them would be inventing numbers.
 */
export type ScoringProfile = {
	name: string;
	/** Attribute index (as a string key) to weight. */
	weights: Record<string, number>;
};

/** How far through parsing the backend is, as it reports it. */
export type ParseProgress = {
	/** 0 to 1. */
	fraction: number;
	/** What the backend is doing, phrased for the user. */
	label: string;
};

/** The editor's name for each of the 52 non-player attributes, in storage
 * order. Two come back empty: the pre-game editor leaves those rows unnamed
 * itself, so Gilet shows the slot number rather than inventing a label. */
export function staffAttributeNames(): Promise<string[]> {
	return invoke<string[]>('staff_attribute_names');
}

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

/** The running app's version from the bundle, so the UI can prove which
 * binary is actually running — a stale install looks identical otherwise. */
export function appVersion(): Promise<string> {
	return getVersion();
}

/**
 * Adds or removes a player on a shortlist inside the save file itself.
 * The backend refuses to touch the file without a `.gilet.bak` sibling
 * holding the untouched original. `list` null targets the unnamed default
 * list; `date` is the save's own current date as [year, month, day].
 */
export function editGameShortlist(
	path: string,
	list: string | null,
	eid: number,
	add: boolean,
	date: [number, number, number]
): Promise<number> {
	return invoke<number>('edit_game_shortlist', { path, list, eid, add, date });
}

/** Empties one in-save shortlist, leaving the list itself. Resolves to
 * whether anything was removed. */
export function clearGameShortlist(path: string, list: string | null): Promise<boolean> {
	return invoke<boolean>('clear_game_shortlist', { path, list });
}

/** Adds many players to one in-save shortlist in a single save rewrite.
 * Resolves to how many were actually added — already-listed players skip. */
export function addPlayersToGameShortlist(
	path: string,
	list: string | null,
	eids: number[],
	date: [number, number, number]
): Promise<number> {
	return invoke<number>('add_players_to_game_shortlist', { path, list, eids, date });
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

export function loadProfiles(): Promise<ScoringProfile[]> {
	return invoke<ScoringProfile[]>('load_profiles');
}

export function saveProfiles(profiles: ScoringProfile[]): Promise<void> {
	return invoke('save_profiles', { profiles });
}
