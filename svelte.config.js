import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
	preprocess: vitePreprocess(),
	kit: {
		// Tauri serves a static bundle from disk, so the whole app prerenders to
		// a single shell and runs client-side. There is no server.
		adapter: adapter({ fallback: 'index.html' })
	}
};
