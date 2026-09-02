<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	interface TrashedItem { id: string; item_type: string; name: string; }
	let items = $state<TrashedItem[]>([]);

	async function loadTrash() {
		try { items = await invoke('get_trash'); } 
		catch (e) { console.error(e); }
	}

	async function restoreItem(id: string, type: string) {
		await invoke('restore_item', { id, itemType: type });
		await loadTrash();
		if (type === 'folder') location.reload();
	}

	async function emptyTrash() {
		if (confirm('Are you absolutely sure? This will permanently delete all files in the trash from your drive.')) {
			await invoke('empty_trash');
			await loadTrash();
		}
	}

	onMount(loadTrash);
</script>

<div class="flex h-full flex-col bg-surface-chat">
	<div class="flex h-12 items-center justify-between border-b border-surface-divider px-4 font-bold shadow-sm">
		<span class="text-white flex items-center gap-2">
			<svg class="h-5 w-5 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
			Trash
		</span>
		{#if items.length > 0}
			<button onclick={emptyTrash} class="rounded bg-red-500/10 px-3 py-1 text-sm text-red-400 transition-colors hover:bg-red-500/20">Empty Trash</button>
		{/if}
	</div>

	<div class="flex-1 overflow-y-auto p-4">
		{#if items.length === 0}
			<div class="flex h-full items-center justify-center text-gray-500">Trash is empty</div>
		{:else}
			<div class="space-y-2">
				{#each items as item}
					<div class="flex items-center justify-between rounded bg-surface-active px-4 py-3 border border-surface-divider">
						<div>
							<span class="text-xs font-bold uppercase text-gray-500">{item.item_type}</span>
							<p class="mt-1 text-sm text-gray-200">{item.name}</p>
						</div>
						<button onclick={() => restoreItem(item.id, item.item_type)} class="rounded bg-surface-input px-3 py-1.5 text-sm text-indigo-400 hover:bg-surface-hover">Restore</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>