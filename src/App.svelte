<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { tools } from './lib/stores';
  import { currentRoute } from './lib/router';
  import NavBar from './lib/components/NavBar.svelte';
  import Home from './lib/pages/Home.svelte';
  import ToolView from './lib/pages/ToolView.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import type { Route } from './lib/router';

  let route = $state<Route>({ page: 'home', params: {} });

  onMount(async () => {
    try {
      const result = await invoke<any[]>('get_tools');
      tools.set(result);
    } catch (e) {
      console.error('加载工具列表失败:', e);
    }
  });

  $effect(() => {
    const unsub = currentRoute.subscribe((r) => {
      route = r;
    });
    return unsub;
  });
</script>

<NavBar />
<main>
  {#if route.page === 'home'}
    <Home />
  {:else if route.page === 'tool'}
    <ToolView toolId={route.params.id} />
  {:else}
    <EmptyState
      icon="🌐"
      title="页面不存在"
      description="请检查链接是否正确"
    />
    <div style="text-align: center; padding: 16px;">
      <a href="#/" style="color: var(--accent);">返回主页</a>
    </div>
  {/if}
</main>

<style>
  main {
    min-height: calc(100vh - 60px);
  }
</style>
