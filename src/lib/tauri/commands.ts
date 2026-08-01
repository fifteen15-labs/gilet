/**
 * The only module that talks to the Rust side. Everything else imports these
 * functions, so the backend can be stubbed in one place for tests.
 */
import { invoke } from '@tauri-apps/api/core';

export type Player = {
	id: number;
	name: string;
	born: string;
	age: number;
	/** null until Current Ability is located in the save format. */
	ability: number | null;
	/** null until Potential Ability is located in the save format. */
	potential: number | null;
};

export type SaveSummary = {
	path: string;
	players: Player[];
	frames: number;
	decompressed_bytes: number;
	parse_millis: number;
};

export type Shortlist = {
	name: string;
	players: string[];
};

export function openSave(path: string): Promise<SaveSummary> {
	const now = new Date();
	// Ages are computed against the user's clock, not the build machine's.
	const today = [now.getFullYear(), now.getMonth() + 1, now.getDate()];
	return invoke<SaveSummary>('open_save', { path, today });
}

export function exportCsv(path: string, rows: Player[]): Promise<void> {
	return invoke('export_csv', { path, rows });
}

export function loadShortlists(): Promise<Shortlist[]> {
	return invoke<Shortlist[]>('load_shortlists');
}

export function saveShortlists(lists: Shortlist[]): Promise<void> {
	return invoke('save_shortlists', { lists });
}
