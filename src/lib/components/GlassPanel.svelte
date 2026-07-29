<script lang="ts">
  let {
    padding = '24px',
    class: className = '',
    children,
  }: {
    padding?: string;
    class?: string;
    children?: import('svelte').Snippet;
  } = $props();
</script>

<div class="glass-panel {className}" style="padding: {padding}">
  {#if children}
    {@render children()}
  {/if}
</div>

<style>
  .glass-panel {
    background: var(--glass-bg);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-glass);
    transition: background var(--transition);
  }
</style>
