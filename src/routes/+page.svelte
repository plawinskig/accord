<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';

	let workspace: string | null = null;
	let loading = true;
	let dbConnected = false;
	let error = '';

	// connects to the database and runs the application
	async function initApp(path: string) {
		try {
			await invoke('connect_to_db', { workspacePath: path });
			dbConnected = true;
		} catch (err) {
			error = String(err);
		}
	}

	// after loading the interface ask if the config already exists
	onMount(async () => {
		try {
			workspace = await invoke('get_workspace');
			if (workspace) {
				await initApp(workspace);
			}
		} catch (err) {
			error = String(err);
		} finally {
			loading = false;
		}
	});

	// opens the system folder selection window
	async function selectFolder() {
		try {
			const selectedPath = await open({
				directory: true,
				multiple: false,
				title: 'Select Workspace Folder'
			});

			if (selectedPath) {
				// send the selected path to Rust (then create folders and save the config)
				await invoke('set_workspace', { path: selectedPath });
				workspace = selectedPath as string;
				await initApp(workspace);
				loading = false;
			}
		} catch (err) {
			error = String(err);
			loading = false;
		}
	}
</script>

<main class="flex h-screen w-screen items-center justify-center bg-[#1e1f22] text-white">
	{#if loading}
		<div class="text-gray-400">Loading...</div>
	{:else if error}
		<div class="text-red-400">Error: {error}</div>
	{:else if !workspace}
		<!-- welcome screen (no workspace) -->
		<div class="flex max-w-md flex-col items-center rounded-lg bg-[#2b2d31] p-8 text-center shadow-lg">
			<h1 class="mb-4 text-3xl font-bold text-white">Welcome to Accord</h1>
			<p class="mb-8 text-sm text-gray-300">
				To get started, choose a local folder where all your notes, databases, and attachments will be securely stored.
			</p>
			<button
				on:click={selectFolder}
				class="rounded-md bg-indigo-500 px-6 py-3 font-semibold text-white transition-colors hover:bg-indigo-600 focus:outline-none"
			>
				Select Workspace
			</button>
		</div>
	{:else if dbConnected}
		<!-- db works -->
		<div class="text-center">
			<h1 class="mb-2 text-2xl font-bold text-green-400">System Ready & Database Connected!</h1>
			<p class="text-gray-400">{workspace}</p>
			<p class="mt-4 text-sm text-gray-500">The core engine is running. Next up: building the UI.</p>
		</div>
	{/if}
</main>