<script lang="ts">
  import { tools } from '../stores';
  import GlassPanel from '../components/GlassPanel.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { navigateTo } from '../router';
  import type { Tool } from '../types';

  let {
    toolId,
  }: {
    toolId: string;
  } = $props();

  let allTools = $state<Tool[]>([]);
  let found = $state<Tool | null>(null);

  $effect(() => {
    const unsub = tools.subscribe((t) => {
      allTools = t;
      found = t.find((tool) => tool.id === toolId) ?? null;
    });
    return unsub;
  });
</script>

<div class="tool-view">
  {#if found}
    <GlassPanel padding="32px">
      <div class="tool-header">
        <span class="tool-icon">{found.icon}</span>
        <div>
          <h2>{found.name}</h2>
          <p class="tool-meta">{found.description}</p>
        </div>
      </div>
      <div class="tool-body">
        <p class="tool-placeholder">工具功能开发中…</p>
      </div>
    </GlassPanel>
  {:else}
    <EmptyState
      icon="❓"
      title="工具未找到"
      description="可能该工具尚未添加，或链接有误"
    />
    <div class="tool-view-back">
      <button class="back-btn" onclick={() => navigateTo('/')}>
        ← 返回主页
      </button>
    </div>
  {/if}
</div>

<style>
  .tool-view {
    max-width: 800px;
    margin: 0 auto;
    padding: 32px 24px;
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 24px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--glass-border);
  }

  .tool-icon {
    font-size: 36px;
  }

  .tool-meta {
    font-size: 14px;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .tool-body {
    min-height: 200px;
  }

  .tool-placeholder {
    color: var(--text-muted);
    text-align: center;
    padding: 60px 0;
    font-size: 15px;
  }

  .tool-view-back {
    display: flex;
    justify-content: center;
    margin-top: 16px;
  }

  .back-btn {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    padding: 8px 20px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 14px;
    transition: background var(--transition);
  }

  .back-btn:hover {
    background: var(--glass-hover);
  }
</style>
