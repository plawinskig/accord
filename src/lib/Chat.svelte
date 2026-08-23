<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { getCurrentWebview } from '@tauri-apps/api/webview';
	import { onMount, onDestroy } from 'svelte';
	import { uiState } from '$lib/state.svelte';

	interface Attachment {
		id: string;
		original_name: string;
		operation_type: 'COPY' | 'MOVE' | 'LINK';
		local_path: string;
		mime_type: string;
	}

	interface Note {
		id: string;
		content: string;
		created_at: string;
		updated_at: string;
		attachments: Attachment[] | null;
	}

	interface PendingFile {
		id: string;
		name: string;
		type: 'blob' | 'path';
		mimeType: string;
		data?: Blob;
		path?: string;
		operation?: 'COPY' | 'MOVE' | 'LINK';
	}

	let notes = $state<Note[]>([]);
	let newNoteContent = $state('');
	let pendingFiles = $state<PendingFile[]>([]);

	// True while an OS-level file drag is hovering over the window, used for
	// a visual drop-zone highlight (there is no DOM dragover event to hook
	// into here - see the onDragDropEvent listener below).
	let isDraggingOver = $state(false);
	
	// Track the edit status
	let editingId = $state<string | null>(null);
	let editContent = $state('');

	// Keep a reference to the notes container for automatic scrolling
	let chatContainer = $state<HTMLElement>();

	// Focus an element while bypassing a11y_autofocus
	function focusOnMount(node: HTMLElement) {
		node.focus();
	}

	// Respond to `uiState.activeFolderId` changes and load the notes
	$effect(() => {
		if (uiState.activeFolderId) {
			loadNotes(uiState.activeFolderId);
			editingId = null;
			pendingFiles = [];
		}
	});

	// Scroll to the bottom when the list of notes changes
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
				setTimeout(() => { uiState.highlightNoteId = null; }, 2500);
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

	// On Linux, ask the backend for clipboard contents when WebKitGTK does not
	// expose an image through e.clipboardData.items. Block the default paste when
	// the backend returns an image; otherwise let handlePaste process the data.
	async function handleKeydownPasteFallback(e: KeyboardEvent) {
		if (!((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v')) return;

		try {
			const base64Png = await invoke<string | null>('read_clipboard_image');
			if (base64Png) {
				e.preventDefault();
				const byteChars = atob(base64Png);
				const byteNumbers = new Array(byteChars.length);
				for (let i = 0; i < byteChars.length; i++) {
					byteNumbers[i] = byteChars.charCodeAt(i);
				}
				const blob = new Blob([new Uint8Array(byteNumbers)], { type: 'image/png' });

				pendingFiles = [...pendingFiles, {
					id: Math.random().toString(),
					name: `Screenshot_${new Date().getTime()}.png`,
					type: 'blob',
					mimeType: 'image/png',
					data: blob,
					operation: 'COPY'
				}];
			}
			// Do nothing when base64Png === null so the paste event can run normally
		} catch (err) {
			console.error('Failed to read the native clipboard:', err);
			// Let handlePaste run normally when an error occurs
		}
	}

	// Handle clipboard input (Ctrl+V)
	function handlePaste(e: ClipboardEvent) {
		if (!e.clipboardData) return;
		let handled = false;

		// Check for a physical image and use Array.from because WebKit on Linux
		// sometimes iterates incorrectly over .items
		const items = Array.from(e.clipboardData.items || []);
		
		for (const item of items) {
			if (item.kind === 'file' && item.type.startsWith('image/')) {
				const blob = item.getAsFile();
				if (blob) {
					pendingFiles = [...pendingFiles, {
						id: Math.random().toString(),
						name: `Screenshot_${new Date().getTime()}.png`,
						type: 'blob',
						mimeType: blob.type,
						data: blob,
						operation: 'COPY'
					}];
					handled = true;
				}
			}
		}

		// Read the clipboard as text when it is not a screenshot
		if (!handled) {
			const plainText = e.clipboardData.getData('text/plain') || '';
			const uriList = e.clipboardData.getData('text/uri-list') || '';
			
			// Combine all clipboard data so nothing is missed
			const combinedText = `${plainText}\n${uriList}`;
			
			// Split the data into lines and clean each line individually
			const lines = combinedText.split(/[\r\n]+/).map(line => line.trim()).filter(line => line.length > 0);

			for (let line of lines) {
				// Remove quotation marks that Linux tends to add
				line = line.replace(/^['"]|['"]$/g, '');
				
				// Check whether the value resembles a system path or file URL
				if (line.startsWith('file://') || line.startsWith('/')) {
					
					let decodedPath = line.replace(/^file:\/\//i, '');
					try { 
						decodedPath = decodeURIComponent(decodedPath); 
					} catch(err) {}
					
					const name = decodedPath.split(/[/\\]/).pop() || 'Unknown';
					
					pendingFiles = [...pendingFiles, {
						id: Math.random().toString(),
						name: name,
						type: 'path',
						mimeType: 'application/octet-stream',
						path: decodedPath,
						operation: 'COPY'
					}];
					handled = true;
				}
			}
		}

		// Prevent the browser from pasting a captured file or image as text
		if (handled) {
			e.preventDefault();
			e.stopPropagation();
		}
	}

	// Handle native OS-level drag & drop.
	// Tauri v2 intercepts window drag-and-drop before it ever reaches the DOM
	// (window-level `dragDropEnabled` defaults to true), so `ondrop`/`ondragover`
	// on a <div> never fire with real files. Instead we listen to Tauri's own
	// webview event, which also gives us real filesystem paths - letting the
	// user pick COPY/MOVE/LINK exactly like the file picker does, instead of
	// only ever being able to copy raw bytes.
	let unlistenDrop: (() => void) | undefined;

	onMount(() => {
		(async () => {
			unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
				if (event.payload.type === 'enter' || event.payload.type === 'over') {
					isDraggingOver = true;
				} else if (event.payload.type === 'drop') {
					isDraggingOver = false;
					handleNativeDrop(event.payload.paths);
				} else if (event.payload.type === 'leave') {
					isDraggingOver = false;
				}
			});
		})();
	});

	onDestroy(() => {
		unlistenDrop?.();
	});

	// Reusable COPY/MOVE/LINK chooser, replacing the native `prompt()` text box
	// with an in-app dialog. `askOperation` resolves once the user picks a
	// button or dismisses the dialog (Escape / backdrop / Cancel -> null).
	let opDialog = $state<{ count: number; resolve: (op: 'COPY' | 'MOVE' | 'LINK' | null) => void } | null>(null);

	function askOperation(count: number): Promise<'COPY' | 'MOVE' | 'LINK' | null> {
		return new Promise((resolve) => {
			opDialog = { count, resolve };
		});
	}

	function chooseOperation(op: 'COPY' | 'MOVE' | 'LINK' | null) {
		opDialog?.resolve(op);
		opDialog = null;
	}

	async function handleNativeDrop(paths: string[]) {
		if (!paths || paths.length === 0) return;

		const operation = await askOperation(paths.length);
		if (!operation) return;

		for (const path of paths) {
			const name = path.split(/[/\\]/).pop() || 'Unknown';
			pendingFiles = [...pendingFiles, {
				id: Math.random().toString(),
				name,
				type: 'path',
				mimeType: 'application/octet-stream',
				path,
				operation
			}];
		}
	}

	// Handle the file picker dialog
	async function openFilePicker() {
		try {
			const selectedPath = await open({ multiple: false, title: 'Attach File' });
			if (selectedPath) {
				const operation = await askOperation(1);
				if (!operation) return;

				const name = typeof selectedPath === 'string' ? selectedPath.split(/[/\\]/).pop() : 'Unknown';
				pendingFiles = [...pendingFiles, {
					id: Math.random().toString(),
					name: name || 'Unknown',
					type: 'path',
					mimeType: 'application/octet-stream',
					path: selectedPath as string,
					operation
				}];
			}
		} catch (e) {
			console.error('Failed to select file', e);
		}
	}

	function removePending(id: string) {
		pendingFiles = pendingFiles.filter(p => p.id !== id);
	}

	async function sendNote(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			
			if (newNoteContent.trim() === '' && pendingFiles.length === 0) return;

			try {
				// Create a note
				const note: Note = await invoke('create_note', { 
					folderId: uiState.activeFolderId, 
					content: newNoteContent.trim() 
				});
				
				// Upload attachments to the new note
				for (const file of pendingFiles) {
					if (file.type === 'blob' && file.data) {
						const buffer = await file.data.arrayBuffer();
						await invoke('attach_blob', { 
							noteId: note.id, 
							// Pass the entire Uint8Array instead of splitting it into individual values
							bytes: new Uint8Array(buffer), 
							originalName: file.name, 
							mimeType: file.mimeType 
						});
					} else if (file.type === 'path' && file.path) {
						const payload = { 
							noteId: note.id, 
							sourcePath: file.path, 
							originalName: file.name, 
							mimeType: file.mimeType 
						};
						
						if (file.operation === 'COPY') await invoke('attach_file_copy', payload);
						else if (file.operation === 'MOVE') await invoke('attach_file_move', payload);
						else if (file.operation === 'LINK') await invoke('attach_file_link', payload);
					}
				}

				// Clear the input and pending files, then reload the notes
				newNoteContent = '';
				pendingFiles = [];
				await loadNotes(uiState.activeFolderId as string);
				
			} catch (e) {
				console.error('Failed to send note with attachments', e);
			}
		}
	}

	// Generate URLs for the accord:// protocol
	function getAttachmentUrl(att: Attachment) {
		// Use `encodeURIComponent` to safely pass spaces and other filename characters
		if (att.operation_type === 'LINK') {
			return `accord://link/${encodeURIComponent(att.local_path)}`;
		}

		return `accord://local/${encodeURIComponent(att.local_path)}`;
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

	// Start editing
	function startEdit(note: Note) {
		editingId = note.id;
		editContent = note.content;
	}

	// Cancel editing
	function cancelEdit() {
		editingId = null;
		editContent = '';
	}

	// Save the edit
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
	class="relative flex h-full flex-col bg-surface-chat transition-colors {isDraggingOver ? 'ring-2 ring-inset ring-indigo-500' : ''}"
>
	{#if isDraggingOver}
		<div class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-indigo-500/10">
			<span class="rounded bg-surface-base px-4 py-2 text-sm font-medium text-indigo-400 shadow">Drop file to attach</span>
		</div>
	{/if}
	{#if uiState.activeFolderId}
		<div class="flex h-12 items-center border-b border-surface-divider px-4 font-bold shadow-sm">
			<span class="mr-2 text-xl text-gray-500">#</span>
			<span class="text-white">{uiState.activeFolderName}</span>
		</div>

		<div bind:this={chatContainer} class="flex-1 space-y-4 overflow-y-auto p-4">
			{#each notes as note}
				<div id="note-{note.id}" class="group relative flex flex-col rounded-md px-2 py-1 transition-all duration-1000 {uiState.highlightNoteId === note.id ? 'bg-indigo-500/20 ring-1 ring-indigo-500' : 'hover:bg-surface-hover'}">
					<div class="mb-1 flex items-center justify-between">
						<span class="text-xs font-medium text-gray-500">{note.created_at}</span>
						
						<!-- Show editing controls -->
						<div class="absolute -top-3 right-4 hidden space-x-2 rounded-md border border-surface-divider bg-surface-chat px-2 py-1 shadow-sm group-hover:flex">
							<button onclick={() => startEdit(note)} class="text-gray-400 hover:text-indigo-400" title="Edit">
								<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
							</button>
							<button onclick={() => deleteNote(note.id)} class="text-gray-400 hover:text-red-400" title="Delete">
								<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
							</button>
						</div>
					</div>

					{#if editingId === note.id}
						<textarea bind:value={editContent} onkeydown={saveEdit} use:focusOnMount class="min-h-15 w-full resize-none rounded bg-surface-input p-2 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500" placeholder="Press Enter to save"></textarea>
					{:else}
						<p class="whitespace-pre-wrap text-sm text-gray-200">{note.content}</p>
					{/if}

					<!-- Render attachments -->
					{#if note.attachments && note.attachments.length > 0}
						<div class="mt-2 flex flex-wrap gap-2">
							{#each note.attachments as att}
								<!-- Verify image attachments -->
								{#if att.mime_type.startsWith('image/') || /\.(png|jpe?g|gif|webp)$/i.test(att.original_name)}
									<a href={getAttachmentUrl(att)} target="_blank" rel="noopener noreferrer" class="block overflow-hidden rounded-md border border-surface-divider max-w-75">
										<img src={getAttachmentUrl(att)} alt={att.original_name} class="h-auto w-full object-cover" />
									</a>
								{:else}
									<a href={getAttachmentUrl(att)} target="_blank" rel="noopener noreferrer" class="flex items-center space-x-2 rounded-md bg-surface-input px-3 py-2 text-sm text-indigo-400 hover:bg-surface-hover border border-surface-divider">
										<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"/></svg>
										<span class="truncate max-w-50">{att.original_name}</span>
										<span class="text-xs text-gray-500">({att.operation_type})</span>
									</a>
								{/if}
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Render the input field from the staging area -->
		<div class="p-4 pt-0">
			<div class="flex flex-col rounded-lg bg-surface-input border border-surface-divider focus-within:border-indigo-500">
				
				<!-- Show files waiting to be attached -->
				{#if pendingFiles.length > 0}
					<div class="flex flex-wrap gap-2 border-b border-surface-divider p-3">
						{#each pendingFiles as pFile}
							<div class="flex items-center space-x-2 rounded bg-surface-chat px-2 py-1 text-sm text-gray-300">
								<span class="truncate max-w-37.5">{pFile.name}</span>
								<span class="text-xs font-bold text-indigo-500">[{pFile.operation}]</span>
								<button onclick={() => removePending(pFile.id)} class="text-gray-500 hover:text-red-400 ml-2">×</button>
							</div>
						{/each}
					</div>
				{/if}

				<div class="flex items-center">
					<!-- Show the paperclip button -->
					<button onclick={openFilePicker} class="pl-4 text-gray-400 hover:text-indigo-400 transition-colors" title="Attach file">
						<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"/></svg>
					</button>

					<!-- Listen for clipboard events: onpaste for text and paths, and onkeydown for native Linux image reads -->
					<textarea
						bind:value={newNoteContent}
						onkeydown={(e) => { handleKeydownPasteFallback(e); sendNote(e); }}
						onpaste={handlePaste}
						placeholder="Message #{uiState.activeFolderName} (Ctrl+V to paste image)"
						class="max-h-[50vh] w-full resize-none bg-transparent px-4 py-3 text-sm text-gray-200 placeholder-gray-500 focus:outline-none"
						rows="1"
					></textarea>
				</div>
			</div>
		</div>

	{:else}
		<div class="flex h-full flex-col items-center justify-center text-gray-500">
			<svg class="mb-4 h-16 w-16 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
			<p class="text-lg font-medium">Select a channel to view notes</p>
		</div>
	{/if}
</div>

{#if opDialog}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
		onclick={() => chooseOperation(null)}
		onkeydown={(e) => { if (e.key === 'Escape') chooseOperation(null); }}
	>
		<div
			class="w-80 rounded-lg border border-surface-divider bg-surface-sidebar p-4 shadow-xl"
			onclick={(e) => e.stopPropagation()}
		>
			<h3 class="mb-1 text-sm font-semibold text-white">
				Attach {opDialog.count === 1 ? 'file' : `${opDialog.count} files`}
			</h3>
			<p class="mb-4 text-xs text-gray-400">Choose how the file should be attached.</p>

			<div class="space-y-2">
				<button
					onclick={() => chooseOperation('COPY')}
					class="w-full rounded-md bg-surface-input px-3 py-2 text-left text-sm text-gray-200 transition-colors hover:bg-surface-active hover:text-white"
				>
					<span class="block font-medium">Copy</span>
					<span class="block text-xs text-gray-400">Keep the original, store a copy in the workspace</span>
				</button>
				<button
					onclick={() => chooseOperation('MOVE')}
					class="w-full rounded-md bg-surface-input px-3 py-2 text-left text-sm text-gray-200 transition-colors hover:bg-surface-active hover:text-white"
				>
					<span class="block font-medium">Move</span>
					<span class="block text-xs text-gray-400">Move the original file into the workspace</span>
				</button>
				<button
					onclick={() => chooseOperation('LINK')}
					class="w-full rounded-md bg-surface-input px-3 py-2 text-left text-sm text-gray-200 transition-colors hover:bg-surface-active hover:text-white"
				>
					<span class="block font-medium">Link</span>
					<span class="block text-xs text-gray-400">Reference the file in place, don't copy it</span>
				</button>
			</div>

			<button
				onclick={() => chooseOperation(null)}
				class="mt-3 w-full rounded-md px-3 py-1.5 text-center text-xs text-gray-400 hover:text-gray-200"
			>
				Cancel
			</button>
		</div>
	</div>
{/if}