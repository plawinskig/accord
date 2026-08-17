<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { uiState } from '$lib/state.svelte';

	interface Note {
		id: string;
		content: string;
		created_at: string;
		updated_at: string;
	}

	let notes = $state<Note[]>([]);
	let newNoteContent = $state('');
	
	// edit status
	let editingId = $state<string | null>(null);
	let editContent = $state('');

	// reference to the notes container (for automatic scrolling)
	let chatContainer = $state<HTMLElement>();

	// action to focus an element (bypasses a11y_autofocus)
	function focusOnMount(node: HTMLElement) {
		node.focus();
	}

    // automatically respond to `uiState.activeFolderId` change and load new notes
	$effect(() => {
		if (uiState.activeFolderId) {
			loadNotes(uiState.activeFolderId);
			editingId = null; // close any editing when changing the folder
		}
	});

    // automatic scroll to the bottom when the list of notes changes
	$effect(() => {
		if (notes.length >= 0 && chatContainer) {
			chatContainer.scrollTop = chatContainer.scrollHeight;
		}
	});

	$effect(() => {
		if (uiState.highlightNoteId) {
			const el = document.getElementById(`note-${uiState.highlightNoteId}`);
			if (el) {
				el.scrollIntoView({ behavior: 'smooth', block: 'center' });
				setTimeout(() => {
					uiState.highlightNoteId = null;
				}, 2500);
			}
		}
	});

	async function loadNotes(folderId: string) {
		try {
			notes = await invoke('get_notes', { folderId });
		} catch (e) {
			console.error('Failed to load notes', e);
		}
	}

	async function sendNote(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey && newNoteContent.trim() !== '') {
			event.preventDefault(); // block new line
			try {
				await invoke('create_note', { 
					folderId: uiState.activeFolderId, 
					content: newNoteContent.trim() 
				});
				newNoteContent = '';
				await loadNotes(uiState.activeFolderId as string);
			} catch (e) {
				console.error('Failed to send note', e);
			}
		}
	}

	async function deleteNote(id: string) {
		if (confirm('Are you sure you want to delete this note?')) {
			try {
				await invoke('soft_delete_note', { id });
				await loadNotes(uiState.activeFolderId as string);
			} catch (e) {
				console.error('Failed to delete note', e);
			}
		}
	}

	// start editing
	function startEdit(note: Note) {
		editingId = note.id;
		editContent = note.content;
	}

	// cancel editing
	function cancelEdit() {
		editingId = null;
		editContent = '';
	}

	// save the edit
	async function saveEdit(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey && editContent.trim() !== '') {
			event.preventDefault();
			try {
				await invoke('update_note', { id: editingId, content: editContent.trim() });
				editingId = null;
				await loadNotes(uiState.activeFolderId as string);
			} catch (e) {
				console.error('Failed to update note', e);
			}
		} else if (event.key === 'Escape') {
			cancelEdit();
		}
	}
</script>

<div class="flex h-full flex-col bg-[#313338]">
	{#if uiState.activeFolderId}
		<!-- chat header -->
		<div class="flex h-12 items-center border-b border-[#2b2d31] px-4 font-bold shadow-sm">
			<span class="mr-2 text-xl text-gray-500">#</span>
			<span class="text-white">{uiState.activeFolderName}</span>
		</div>

        <!-- notes area -->
		<div bind:this={chatContainer} class="flex-1 space-y-4 overflow-y-auto p-4">
			{#if notes.length === 0}
				<div class="flex h-full flex-col items-center justify-center text-center">
					<div class="mb-4 rounded-full bg-[#2b2d31] p-6 text-gray-500">
						<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"/></svg>
					</div>
					<h2 class="text-2xl font-bold text-gray-300">Welcome to #{uiState.activeFolderName}!</h2>
					<p class="mt-2 text-gray-400">This is the start of this channel.</p>
				</div>
			{/if}

            <!-- note rendering -->
			{#each notes as note}
				<div 
                    id="note-{note.id}" 
                    class="group relative flex flex-col rounded-md px-2 py-1 transition-all duration-1000 {uiState.highlightNoteId === note.id ? 'bg-indigo-500/20 ring-1 ring-indigo-500' : 'hover:bg-[#2b2d31]'}"
                >
					
					<!-- top note bar (date and edit icons hidden under hover) -->
					<div class="mb-1 flex items-center justify-between">
						<span class="text-xs font-medium text-gray-500">
							{note.created_at}
							{#if note.created_at !== note.updated_at}
								<span class="ml-1 italic">(edited)</span>
							{/if}
						</span>
						
                        <!-- edit/delete controls -->
						<div class="absolute -top-3 right-4 hidden space-x-2 rounded-md border border-[#1e1f22] bg-[#313338] px-2 py-1 shadow-sm group-hover:flex">
							<button onclick={() => startEdit(note)} class="text-gray-400 hover:text-indigo-400" title="Edit">
								<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
							</button>
							<button onclick={() => deleteNote(note.id)} class="text-gray-400 hover:text-red-400" title="Delete">
								<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
							</button>
						</div>
					</div>

                    <!-- note content (or edit input) -->
					{#if editingId === note.id}
						<textarea
							bind:value={editContent}
							onkeydown={saveEdit}
							use:focusOnMount
							class="min-h-15 w-full resize-none rounded bg-[#383a40] p-2 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
							placeholder="Press Enter to save, Escape to cancel"
						></textarea>
						<span class="mt-1 text-xs text-gray-500">Escape to cancel • Enter to save</span>
					{:else}
						<p class="whitespace-pre-wrap text-sm text-gray-200">{note.content}</p>
					{/if}
				</div>
			{/each}
		</div>

		<!-- field for entering a new note at the bottom -->
		<div class="p-4 pt-0">
			<div class="relative flex items-center rounded-lg bg-[#383a40]">
				<textarea
					bind:value={newNoteContent}
					onkeydown={sendNote}
					placeholder="Message #{uiState.activeFolderName}"
					class="max-h-[50vh] w-full resize-none rounded-lg bg-transparent px-4 py-3 text-sm text-gray-200 placeholder-gray-500 focus:outline-none"
					rows="1"
				></textarea>
			</div>
		</div>

	{:else}
		<!-- screen when no channel is selected -->
		<div class="flex h-full flex-col items-center justify-center text-gray-500">
			<svg class="mb-4 h-16 w-16 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
			<p class="text-lg font-medium">Select a channel to view notes</p>
		</div>
	{/if}
</div>