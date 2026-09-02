<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import Sidebar from '$lib/Sidebar.svelte';
	import Chat from '$lib/Chat.svelte';
	import RightSidebar from '$lib/RightSidebar.svelte';

	let workspace: string | null = null;
	let loading = true;
	let dbConnected = false;
	let error = '';

	// Connect to the database and run the application
	async function initApp(path: string) {
		try {
			await invoke('connect_to_db', { workspacePath: path });
			dbConnected = true;
		} catch (err) {
			error = String(err);
		}
	}

	// Ask whether the configuration already exists after loading the interface
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

	// Open the system folder selection window
	async function selectFolder() {
		try {
			const selectedPath = await open({
				directory: true,
				multiple: false,
				title: 'Select Workspace Folder'
			});

			if (selectedPath) {
				// Send the selected path to Rust, then create folders and save the configuration
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

<main class="flex h-screen w-screen items-center justify-center bg-surface-base text-white">
	{#if loading}
		<div class="text-gray-400">Loading...</div>
	{:else if error}
		<div class="text-red-400">Error: {error}</div>
	{:else if !workspace}
		<!-- Show the welcome screen when no workspace is configured -->
		<div class="flex max-w-md flex-col items-center rounded-lg bg-surface-sidebar p-8 text-center shadow-lg">
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
		<!-- Render the main application interface -->
		<div class="flex h-screen w-screen overflow-hidden bg-surface-chat text-white">
			
			<!-- Render the left channel and folder column -->
			<Sidebar />
			
			<!-- Render the middle chat column -->
			<div class="flex flex-1 flex-col">
				<Chat />
			</div>

			<!-- Render the right tags and search column -->
			<RightSidebar />
		</div>
	{/if}
</main>