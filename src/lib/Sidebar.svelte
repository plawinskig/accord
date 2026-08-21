<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { uiState } from '$lib/state.svelte';

	interface Folder {
		id: string;
		parent_id: string | null;
		name: string;
	}

	interface TreeNode extends Folder {
		children: TreeNode[];
	}

	let flatFolders = $state<Folder[]>([]);
	let folderTree = $state<TreeNode[]>([]);
	let newFolderName = $state('');

	// get folders from rust
	async function loadFolders() {
		try {
			flatFolders = await invoke('get_folders');
			folderTree = buildTree(flatFolders, null);
		} catch (e) {
			console.error('Failed to load folders', e);
		}
	}

    // recursive function: turns a flat list into a tree
    // by removing orphaned subfolders
	function buildTree(folders: Folder[], parentId: string | null): TreeNode[] {
		return folders
			.filter((f) => f.parent_id === parentId)
			.map((f) => ({
				...f,
				children: buildTree(folders, f.id)
			}));
	}

	// add a root folder from the bar at the bottom
	async function addRootFolder(event: KeyboardEvent) {
		if (event.key === 'Enter' && newFolderName.trim() !== '') {
			try {
				await invoke('create_folder', { name: newFolderName.trim(), parentId: null });
				newFolderName = ''; // clear input
				await loadFolders(); // reload list
			} catch (e) {
				console.error('Failed to create root folder', e);
			}
		}
	}

	// creating a subfolder using the native prompt for speed
	async function addSubFolder(parentId: string) {
		const name = prompt('Enter subchannel name:');
		if (name && name.trim() !== '') {
			try {
				await invoke('create_folder', { name: name.trim(), parentId });
				await loadFolders();
			} catch (e) {
				console.error('Failed to create subfolder', e);
			}
		}
	}

	// delete to trash
	async function deleteFolder(id: string, name: string) {
		if (confirm(`Are you sure you want to delete #${name} and all its contents?`)) {
			try {
				await invoke('soft_delete_folder', { id });
				// if deleted active folder, clear the state
				if (uiState.activeFolderId === id) {
					uiState.activeFolderId = null;
					uiState.activeFolderName = null;
				}
				await loadFolders();
			} catch (e) {
				console.error('Failed to delete folder', e);
			}
		}
	}

	onMount(() => {
		loadFolders();
	});
</script>

<!-- RECURSIVE SNIPPET TO RENDER FOLDER TREE -->
{#snippet folderNode(node: TreeNode, depth: number)}
	<!-- render the folder itself, the indentation increases with each depth -->

	<!-- clicking sets this folder as the active one globally -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
	<div 
		onclick={() => { uiState.activeFolderId = node.id; uiState.activeFolderName = node.name; }}
		class="group flex cursor-pointer items-center justify-between rounded py-1.5 pr-2 transition-colors {uiState.activeFolderId === node.id ? 'bg-surface-active text-white' : 'text-gray-400 hover:bg-surface-hover hover:text-gray-200'}"
		style="padding-left: calc(0.5rem + {depth} * 1rem);"
	>
		<div class="flex flex-1 items-center truncate">
			<span class="mr-2 text-xl {uiState.activeFolderId === node.id ? 'text-gray-300' : 'text-gray-500'}">#</span>
			<span class="truncate font-medium">{node.name}</span>
		</div>
		
		<!-- action buttons (appear on mouse hover) -->
		<div class="hidden items-center space-x-1 group-hover:flex">
			<button 
				onclick={(e) => { e.stopPropagation(); addSubFolder(node.id); }} 
				class="text-gray-400 transition-colors hover:text-green-400" 
				title="Add subchannel"
			>
				<!-- plus icon -->
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
			</button>
			<button 
				onclick={(e) => { e.stopPropagation(); deleteFolder(node.id, node.name); }} 
				class="ml-1 text-gray-400 transition-colors hover:text-red-400" 
				title="Delete channel"
			>
				<!-- trash icon -->
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
			</button>
		</div>
	</div>

	<!-- if the folder has subfolders call the snippet for each subfolder -->
	{#if node.children.length > 0}
		{#each node.children as child}
			{@render folderNode(child, depth + 1)}
		{/each}
	{/if}
{/snippet}

<div class="flex h-full w-60 flex-col bg-surface-sidebar">
	<div class="flex h-12 cursor-pointer items-center border-b border-surface-base px-4 font-bold text-white shadow-sm transition-colors hover:bg-surface-hover">
		Accord Base
	</div>

	<div class="flex-1 overflow-y-auto p-2">
		<!-- start drawing the tree from depth 0 -->
		{#each folderTree as rootFolder}
			{@render folderNode(rootFolder, 0)}
		{/each}
	</div>

	<div class="bg-surface-input-bg p-4">
		<input
			type="text"
			bind:value={newFolderName}
			onkeydown={addRootFolder}
			placeholder="Create root channel..."
			class="w-full rounded bg-surface-base px-3 py-1.5 text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
		/>
	</div>
</div>