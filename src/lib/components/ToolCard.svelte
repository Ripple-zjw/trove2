<script lang="ts">
  import GlassPanel from './GlassPanel.svelte';
  import type { Tool } from '../types';
  import { navigateTo } from '../router';

  let {
    tool,
  }: {
    tool: Tool;
  } = $props();

  function handleClick() {
    navigateTo(`/tool/${tool.id}`);
  }
</script>

<button class="tool-card-wrapper" onclick={handleClick}>
  <GlassPanel padding="20px">
    <div class="tool-card">
      <span class="tool-icon">{tool.icon}</span>
      <div class="tool-info">
        <h3 class="tool-name">{tool.name}</h3>
        <p class="tool-desc">{tool.description}</p>
      </div>
    </div>
  </GlassPanel>
</button>

<style>
  .tool-card-wrapper {
    all: unset;
    cursor: pointer;
    display: block;
    transition: transform var(--transition);
  }

  .tool-card-wrapper:hover {
    transform: translateY(-2px);
  }

  .tool-card-wrapper:hover :global(.glass-panel) {
    background: var(--glass-hover);
  }

  .tool-card {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }

  .tool-icon {
    font-size: 28px;
    line-height: 1;
    flex-shrink: 0;
  }

  .tool-info {
    flex: 1;
    min-width: 0;
  }

  .tool-name {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 4px;
    color: var(--text-primary);
  }

  .tool-desc {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
