<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
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

	// Struktura do "poczekalni" przed wysłaniem
	interface PendingFile {
		id: string; // tymczasowe
		name: string;
		type: 'blob' | 'path';
		mimeType: string;
		data?: Blob;
		path?: string;
		operation?: 'COPY' | 'MOVE' | 'LINK';
	}

	let notes = $state<Note[]>([]);
	let newNoteContent = $state('');
	let pendingFiles = $state<PendingFile[]>([]); // Staging area!
	
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
			editingId = null;
			pendingFiles = []; // Czyścimy kolejkę przy zmianie kanału
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

	// OBSŁUGA SCHOWKA (CTRL+V) - Kompletnie nowa wersja
	function handlePaste(e: ClipboardEvent) {
		if (!e.clipboardData) return;
		let handled = false;

		// 1. Sprawdzamy czy to prawdziwy plik obrazka (Narzędzie wycinania, zrzuty ekranu)
		if (e.clipboardData.files && e.clipboardData.files.length > 0) {
			for (let i = 0; i < e.clipboardData.files.length; i++) {
				const file = e.clipboardData.files[i];
				if (file.type.startsWith('image/')) {
					pendingFiles = [...pendingFiles, {
						id: Math.random().toString(),
						name: `Screenshot_${new Date().getTime()}.png`,
						type: 'blob',
						mimeType: file.type,
						data: file,
						operation: 'COPY'
					}];
					handled = true;
				}
			}
		}

		// 1B. Fallback dla przeglądarek, które podpinają zrzuty pod .items
		if (!handled && e.clipboardData.items) {
			for (let i = 0; i < e.clipboardData.items.length; i++) {
				const item = e.clipboardData.items[i];
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
		}

		// 2. Jeśli to nie obrazek, sprawdzamy czy to ścieżka do pliku (kopiowanie w Linux Nemo)
		if (!handled) {
			// Linux używa uri-list do trzymania plików w schowku
			const uriList = e.clipboardData.getData('text/uri-list');
			const plainText = e.clipboardData.getData('text/plain');
			const textData = uriList || plainText;

			if (textData && (textData.includes('file://') || textData.startsWith('/'))) {
				const lines = textData.split('\n').map(line => line.trim()).filter(line => line.length > 0);
				
				for (const line of lines) {
					if (line.startsWith('file://') || line.startsWith('/')) {
						// Dekodujemy np. %20 z powrotem na spacje
						let decodedPath = line.replace(/^file:\/\//i, '');
						try { decodedPath = decodeURIComponent(decodedPath); } catch(err) {}
						decodedPath = decodedPath.replace(/\r/g, ''); // Usuwa znak nowej linii
						
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
		}

		// Jeśli cokolwiek zrobiliśmy (dodaliśmy plik/obraz) to blokujemy domyślne zachowanie!
		// Dzięki temu ścieżka z pliku nie wklei się ordynarnie do pola tekstowego
		if (handled) {
			e.preventDefault();
		}
	}

	// OBSŁUGA DRAG & DROP (Upuszczanie plików)
	function handleDrop(e: DragEvent) {
		e.preventDefault();
		if (!e.dataTransfer) return;
		
		for (const file of e.dataTransfer.files) {
			pendingFiles = [...pendingFiles, {
				id: Math.random().toString(),
				name: file.name,
				type: 'blob',
				mimeType: file.type || 'application/octet-stream',
				data: file,
				operation: 'COPY'
			}];
		}
	}

	// OBSŁUGA DIALOGU (Spinacz - Pliki z dysku)
	async function openFilePicker() {
		try {
			const selectedPath = await open({ multiple: false, title: 'Attach File' });
			if (selectedPath) {
				const op = prompt("Do you want to COPY, MOVE, or LINK the file?\nType: COPY, MOVE, or LINK", "COPY");
				if (!op) return;
				
				const operation = op.toUpperCase();
				
				if (['COPY', 'MOVE', 'LINK'].includes(operation)) {
					const name = typeof selectedPath === 'string' ? selectedPath.split(/[/\\]/).pop() : 'Unknown';
					pendingFiles = [...pendingFiles, {
						id: Math.random().toString(),
						name: name || 'Unknown',
						type: 'path',
						mimeType: 'application/octet-stream',
						path: selectedPath as string,
						operation: operation as 'COPY' | 'MOVE' | 'LINK'
					}];
				}
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
			
			// Blokada przed pustym strzałem
			if (newNoteContent.trim() === '' && pendingFiles.length === 0) return;

			try {
				// 1. Tworzymy Notatkę!
				const note: Note = await invoke('create_note', { 
					folderId: uiState.activeFolderId, 
					content: newNoteContent.trim() 
				});
				
				// 2. Ładujemy załączniki do nowo utworzonej notatki!
				for (const file of pendingFiles) {
					if (file.type === 'blob' && file.data) {
						const buffer = await file.data.arrayBuffer();
						await invoke('attach_blob', { 
							noteId: note.id, 
							// ZMIANA: Przekazujemy Uint8Array w całości zamiast rozbijać go na milion cyfr do Array!
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

				// 3. Czyścimy
				newNoteContent = '';
				pendingFiles = [];
				await loadNotes(uiState.activeFolderId as string);
				
			} catch (e) {
				console.error('Failed to send note with attachments', e);
			}
		}
	}

	// Funkcja pomocnicza generująca URL naszego protokołu accord://
	function getAttachmentUrl(att: Attachment) {
		// Używamy encodeURIComponent, by bezpiecznie przekazać np. spacje z nazwy pliku
		if (att.operation_type === 'LINK') {
			return `accord://link/${encodeURIComponent(att.local_path)}`;
		}

		return `accord://local/${encodeURIComponent(att.local_path)}`;
	}

	// (Reszta standardowych funkcji: deleteNote, startEdit, cancelEdit, saveEdit bez zmian...)
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

<!-- Kontener musi wyłapywać zdarzenia Drag&Drop -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
	ondrop={handleDrop} 
	ondragover={(e) => e.preventDefault()}
	class="flex h-full flex-col bg-surface-chat"
>
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
						
						<!-- Kontrolki Edycji -->
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

					<!-- RENDEROWANIE ZAŁĄCZNIKÓW! -->
					{#if note.attachments && note.attachments.length > 0}
						<div class="mt-2 flex flex-wrap gap-2">
							{#each note.attachments as att}
								<!-- NIEZAWODNE SPRAWDZANIE OBRAZKA: -->
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

		<!-- Pole Inputu ze STAGING AREA -->
		<div class="p-4 pt-0">
			<div class="flex flex-col rounded-lg bg-surface-input border border-surface-divider focus-within:border-indigo-500">
				
				<!-- Widok poczekalni na pliki -->
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
					<!-- Przycisk Spinacza -->
					<button onclick={openFilePicker} class="pl-4 text-gray-400 hover:text-indigo-400 transition-colors" title="Attach file">
						<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"/></svg>
					</button>

					<!-- Nasłuchujemy schowka na onpaste! -->
					<textarea
						bind:value={newNoteContent}
						onkeydown={sendNote}
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