<script lang="ts">
  import GlassPanel from './GlassPanel.svelte';

  let {
    icon = '🔧',
    title = '暂无工具',
    description = '敬请期待',
  }: {
    icon?: string;
    title?: string;
    description?: string;
  } = $props();
</script>

<div class="empty-wrapper">
  <GlassPanel padding="48px">
    <div class="empty-content">
      <span class="empty-icon">{icon}</span>
      <h2 class="empty-title">{title}</h2>
      <p class="empty-desc">{description}</p>
    </div>
  </GlassPanel>
</div>

<style>
  .empty-wrapper {
    display: flex;
    justify-content: center;
    padding: 60px 20px;
  }

  .empty-content {
    text-align: center;
  }

  .empty-icon {
    font-size: 48px;
    display: block;
    margin-bottom: 16px;
  }

  .empty-title {
    font-size: 20px;
    font-weight: 600;
    margin-bottom: 8px;
    color: var(--text-primary);
  }

  .empty-desc {
    font-size: 14px;
    color: var(--text-secondary);
  }
</style>
