<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let tools: any[] = $state([]);

  onMount(async () => {
    try {
      tools = await invoke('get_tools');
    } catch (e) {
      console.error('Failed to get tools:', e);
    }
  });
</script>

<main>
  <h1>Trove2</h1>
  <p>工具数量: {tools.length}</p>
</main>
