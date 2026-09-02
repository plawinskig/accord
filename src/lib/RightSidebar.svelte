<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { uiState } from '$lib/state.svelte';
	import SearchBar from '$lib/SearchBar.svelte';

	interface Tag {
		id: string;
		name: string;
	}

	let tags = $state<Tag[]>([]);

	// Load the global tag list from Rust
	async function loadTags() {
		try {
			tags = await invoke('get_all_tags');
		} catch (e) {
			console.error('Failed to load tags', e);
		}
	}

	// Click a tag to filter by it; clicking the already-active tag clears
	// the filter. This only updates shared state for now
	function toggleTag(tag: Tag) {
		uiState.activeTagId = uiState.activeTagId === tag.id ? null : tag.id;
	}

	// Delete a tag everywhere it's used (global operation, not per-note)
	async function removeTagGlobally(e: MouseEvent, tag: Tag) {
		e.stopPropagation();
		if (!confirm(`Delete the tag "${tag.name}" everywhere it's used?`)) return;

		try {
			await invoke('delete_tag', { id: tag.id });
			if (uiState.activeTagId === tag.id) {
				uiState.activeTagId = null;
			}
			await loadTags();
		} catch (err) {
			console.error('Failed to delete tag', err);
		}
	}

	onMount(() => {
		loadTags();
	});
</script>

<div class="hidden w-87.5 flex-col border-l border-surface-base bg-surface-sidebar lg:flex lg:flex-col">
	<!-- Search bar header, matching the left sidebar's header height -->
	<div class="flex h-12 items-center justify-end border-b border-surface-base px-4 shadow-sm">
		<SearchBar />
	</div>

	<div class="flex-1 overflow-y-auto p-3">
		<h2 class="mb-2 px-1 text-xs font-semibold uppercase tracking-wide text-gray-500">Tags</h2>

		{#if tags.length === 0}
			<p class="px-1 text-sm text-gray-500">
				No tags yet. Type :: followed by a word in a message to create one.
			</p>
		{:else}
			<div class="flex flex-wrap gap-2">
				{#each tags as tag (tag.id)}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						onclick={() => toggleTag(tag)}
						class="group flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-sm transition-colors {uiState.activeTagId ===
						tag.id
							? 'bg-indigo-500 text-white'
							: 'bg-surface-active text-gray-300 hover:bg-surface-hover'}"
						title={uiState.activeTagId === tag.id
							? 'Click to clear filter'
							: `Filter by "${tag.name}"`}
					>
						<span class="truncate">{tag.name}</span>
						<button
							onclick={(e) => removeTagGlobally(e, tag)}
							class="hidden text-gray-400 transition-colors hover:text-red-300 group-hover:inline"
							title="Delete tag everywhere"
						>
							<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							</svg>
						</button>
					</div>
				{/each}
			</div>
		{/if}

		{#if uiState.activeTagId}
			<button
				onclick={() => (uiState.activeTagId = null)}
				class="mt-4 w-full rounded bg-surface-base px-3 py-1.5 text-xs text-gray-400 transition-colors hover:text-gray-200"
			>
				Clear tag filter
			</button>
		{/if}
	</div>
</div>