<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { uiState } from '$lib/state.svelte';

	let query = $state('');
	let results = $state<any[]>([]);
	let isFocused = $state(false);
	
	// Wait briefly after key release before querying the database
	let debounceTimer: ReturnType<typeof setTimeout>;

	async function performSearch() {
		if (query.trim().length < 2) {
			results = [];
			return;
		}
		try {
			results = await invoke('search_notes', { query: query.trim() });
		} catch (e) {
			console.error('Search failed', e);
			results = [];
		}
	}

	function handleInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(performSearch, 200);
	}

	function selectResult(result: any) {
		// Change the active folder to the note's folder
		uiState.activeFolderId = result.folder_id;
		uiState.activeFolderName = result.folder_name;
		
		// Wait for the chat to render, then specify the note to highlight
		setTimeout(() => {
			uiState.highlightNoteId = result.note_id;
		}, 50);

		// Reset the search state
		query = '';
		results = [];
		isFocused = false;
	}
</script>

<div class="relative w-full max-w-sm">
	<!-- Render the input bar -->
	<div class="relative flex items-center">
		<input
			type="text"
			bind:value={query}
			oninput={handleInput}
			onfocus={() => (isFocused = true)}
			onblur={() => setTimeout(() => (isFocused = false), 150)}
			placeholder="Search notes..."
			class="w-full rounded bg-surface-base px-3 py-1.5 pl-8 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
		/>
		<svg class="absolute left-2.5 h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
		</svg>
	</div>

	<!-- Show the results dropdown -->
	{#if isFocused && query.length >= 2}
		<div class="absolute right-0 top-10 z-50 w-100 overflow-hidden rounded-md border border-surface-base bg-surface-sidebar shadow-xl">
			{#if results.length === 0}
				<div class="p-4 text-center text-sm text-gray-500">No results found.</div>
			{:else}
				<div class="max-h-[60vh] overflow-y-auto py-2">
					{#each results as result}
						<!-- Use onmousedown so it runs before the input loses focus -->
						<button 
							onmousedown={(e) => { e.preventDefault(); selectResult(result); }}
							class="w-full border-b border-surface-base px-4 py-3 text-left transition-colors hover:bg-surface-hover last:border-0"
						>
							<div class="mb-1 flex items-center justify-between">
								<span class="text-xs font-bold text-gray-400">#{result.folder_name}</span>
								<span class="text-xs text-gray-500">{result.created_at.substring(0, 10)}</span>
							</div>
							<!-- Render the highlighted snippet -->
							<div class="text-sm text-gray-300">
								{@html result.snippet}
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>