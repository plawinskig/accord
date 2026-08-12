<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	interface Folder {
		id: string;
		parent_id: string | null;
		name: string;
	}

	let folders = $state<Folder[]>([]);
	let newFolderName = $state('');

	// get folders from rust
	async function loadFolders() {
		try {
			folders = await invoke('get_folders');
		} catch (e) {
			console.error('Failed to load folders', e);
		}
	}

	// send the new folder to rust when enter clicked
	async function addFolder(event: KeyboardEvent) {
		if (event.key === 'Enter' && newFolderName.trim() !== '') {
			try {
				await invoke('create_folder', { name: newFolderName.trim(), parentId: null });
				newFolderName = ''; // clear input
				await loadFolders(); // reload list
			} catch (e) {
				console.error('Failed to create folder', e);
			}
		}
	}

	onMount(() => {
		loadFolders();
	});
</script>

<div class="flex h-full w-60 flex-col bg-[#2b2d31]">
	<!-- server top header -->
	<div class="flex h-12 items-center border-b border-[#1e1f22] px-4 font-bold text-white shadow-sm hover:bg-[#35373c] cursor-pointer transition-colors">
		Accord Base
	</div>

	<!-- folder list -->
	<div class="flex-1 overflow-y-auto p-3">
		{#each folders as folder}
			<div class="group flex cursor-pointer items-center rounded px-2 py-1.5 text-gray-400 transition-colors hover:bg-[#35373c] hover:text-gray-200">
				<!-- channel icon -->
				<span class="mr-2 text-xl text-gray-500">#</span>
				<span class="truncate">{folder.name}</span>
			</div>
		{/each}
	</div>

	<!-- adding a new folder (plain input at the bottom) -->
	<div class="bg-[#232428] p-4">
		<input
			type="text"
			bind:value={newFolderName}
			onkeydown={addFolder}
			placeholder="Create channel..."
			class="w-full rounded bg-[#1e1f22] px-3 py-1.5 text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
		/>
	</div>
</div>