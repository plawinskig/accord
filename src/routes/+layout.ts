// Use adapter-static with an index.html fallback to run the site in SPA mode
// because Tauri does not provide a Node.js server for SSR
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;
