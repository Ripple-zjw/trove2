<script lang="ts">
  import { tools, searchQuery, selectedCategory } from '../stores';
  import ToolCard from '../components/ToolCard.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import type { Tool } from '../types';

  let allTools = $state<Tool[]>([]);
  let query = $state('');
  let category = $state('');

  // 从工具列表动态提取分类，并自动加入"全部"
  let categories = $derived(['全部', ...new Set(allTools.map(t => t.category).filter(Boolean))]);

  $effect(() => {
    const unsubTools = tools.subscribe((t) => { allTools = t; });
    const unsubQuery = searchQuery.subscribe((q) => { query = q; });
    const unsubCat = selectedCategory.subscribe((c) => { category = c; });
    return () => { unsubTools(); unsubQuery(); unsubCat(); };
  });

  let filtered = $derived.by(() => {
    let list = allTools;
    if (category && category !== '全部') {
      list = list.filter((t) => t.category === category);
    }
    if (query) {
      const q = query.toLowerCase();
      list = list.filter(
        (t) =>
          t.name.toLowerCase().includes(q) ||
          t.description.toLowerCase().includes(q)
      );
    }
    return list;
  });

  function selectCategory(cat: string) {
    selectedCategory.set(cat);
  }

  const hasTools = $derived(allTools.length > 0);
  const noResults = $derived(hasTools && filtered.length === 0);
</script>

<div class="home">
  <div class="home-content">
    <div class="home-header">
      <h2 class="home-subtitle">工具集合</h2>
      {#if hasTools}
        <span class="tool-count">{allTools.length} 个工具</span>
      {/if}
    </div>

    {#if hasTools}
      <div class="category-bar">
        {#each categories as cat}
          <button
            class="category-chip"
            class:active={category === cat || (!category && cat === '全部')}
            onclick={() => selectCategory(cat)}
          >
            {cat}
          </button>
        {/each}
      </div>
    {/if}

    {#if noResults}
      <EmptyState
        icon="🔍"
        title="没有匹配的工具"
        description="尝试换个关键词搜索吧"
      />
    {:else if !hasTools}
      <EmptyState icon="🔧" title="还没有工具" description="敬请期待" />
    {:else}
      <div class="tool-grid">
        {#each filtered as tool (tool.id)}
          <ToolCard {tool} />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .home {
    max-width: 960px;
    margin: 0 auto;
    padding: 24px;
  }

  .home-header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 20px;
  }

  .home-subtitle {
    font-size: 24px;
    font-weight: 700;
  }

  .tool-count {
    font-size: 14px;
    color: var(--text-secondary);
  }

  .category-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 24px;
  }

  .category-chip {
    padding: 6px 16px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: 20px;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    transition: all var(--transition);
  }

  .category-chip:hover {
    background: var(--glass-hover);
    color: var(--text-primary);
  }

  .category-chip.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .tool-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }
</style>
