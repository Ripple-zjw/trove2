<script lang="ts">
  import { navigateTo, currentRoute } from '../router';
  import { searchQuery, selectedCategory } from '../stores';

  let query = $state('');
  let route = $state({ page: 'home', params: {} as Record<string, string>});
  let showSearch = $state(false);

  $effect(() => {
    const unsub = currentRoute.subscribe((r) => {
      route = r;
      showSearch = r.page === 'home';
    });
    return unsub;
  });

  function onSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    query = val;
    searchQuery.set(val);
  }

  function goHome() {
    searchQuery.set('');
    selectedCategory.set('');
    query = '';
    navigateTo('/');
  }
</script>

<nav class="navbar">
  <div class="nav-left">
    {#if route.page !== 'home'}
      <button class="nav-back" onclick={goHome}>← 返回</button>
    {/if}
    <button class="nav-title" onclick={goHome}>Trove2</button>
  </div>
  {#if showSearch}
    <div class="nav-search">
      <input
        type="text"
        class="search-input"
        placeholder="搜索工具…"
        bind:value={query}
        oninput={onSearchInput}
      />
    </div>
  {/if}
</nav>

<style>
  .navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 24px;
    background: var(--glass-bg);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border-bottom: 1px solid var(--glass-border);
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .nav-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .nav-title {
    padding: 0;
    border: 0;
    font-size: 20px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent), #f093fb);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    cursor: pointer;
    user-select: none;
  }

  .nav-back {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 14px;
    transition: background var(--transition);
  }

  .nav-back:hover {
    background: var(--glass-hover);
  }

  .nav-search {
    flex: 1;
    max-width: 320px;
  }

  .search-input {
    width: 100%;
    padding: 8px 14px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 14px;
    outline: none;
    transition: border-color var(--transition);
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-input:focus {
    border-color: var(--accent);
  }
</style>
