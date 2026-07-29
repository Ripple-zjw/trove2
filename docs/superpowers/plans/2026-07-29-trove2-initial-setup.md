# Trove2 空壳应用实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 Tauri v2 + Svelte 搭建一个空的毛玻璃风格个人工具集应用，包含主页（搜索/分类/空状态）和工具容器页。

**Architecture:** Rust 后端通过 `#[tauri::command]` 暴露工具列表数据；Svelte 前端通过 `invoke('get_tools')` 获取后渲染毛玻璃风格主页；hash 路由实现页面切换，无需额外依赖。

**Tech Stack:** Tauri v2 (Rust), Svelte 5 (编译型前端), Vite 6, TypeScript

## Global Constraints

- 工具清单硬编码在 Rust 侧（`Vec<Tool>`），通过 IPC `get_tools` 返回
- 前端零框架依赖（无 React/Vue），无第三方路由库
- 毛玻璃风格：深色渐变背景，`backdrop-filter: blur(20px)` 卡片，暗色主题
- 所有文本使用中文（设计文档语言规范）
- 项目标识符：`com.trove2.app`
- 构建时需设置代理 `http://127.0.0.1:7897`（中国大陆网络环境）

## 文件结构预览

```
/Users/ripple/projects/trove2/
├── index.html                    # Svelte 入口 HTML
├── package.json                  # npm 依赖
├── svelte.config.js              # Svelte 编译配置
├── tsconfig.json                 # TS 配置
├── tsconfig.node.json            # Vite 侧 TS 配置
├── vite.config.ts                # Vite + Tauri 配置
├── .gitignore
├── src/
│   ├── main.ts                   # Svelte 挂载入口
│   ├── App.svelte                # 根组件（路由 + 布局）
│   ├── app.css                   # 全局毛玻璃主题
│   ├── vite-env.d.ts
│   └── lib/
│       ├── types.ts              # Tool 接口定义
│       ├── stores.ts             # Svelte stores
│       ├── router.ts             # Hash 路由工具函数
│       ├── components/
│       │   ├── GlassPanel.svelte # 毛玻璃容器（可复用）
│       │   ├── ToolCard.svelte   # 工具卡片
│       │   ├── NavBar.svelte     # 顶栏（标题/搜索/返回）
│       │   └── EmptyState.svelte # 空状态组件
│       └── pages/
│           ├── Home.svelte       # 主页：搜索 + 分类 + 卡片网格
│           └── ToolView.svelte   # 工具容器页 + 未找到处理
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/
    │   └── default.json
    └── src/
        ├── main.rs               # Tauri 入口
        ├── lib.rs                # 模块导出 + 命令注册
        ├── models/
        │   ├── mod.rs
        │   └── tool.rs           # Tool 结构体
        └── commands/
            ├── mod.rs
            └── tools.rs          # get_tools 命令
```

---

### Task 1: 项目脚手架搭建

**文件：**
- Create: `package.json`
- Create: `index.html`
- Create: `svelte.config.js`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `.gitignore`

**说明：** 搭建 npm + Vite + Svelte 5 项目脚手架，不涉及任何应用逻辑。

- [ ] **Step 1: 创建 package.json**

```json
{
  "name": "trove2",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@tauri-apps/cli": "^2.0.0",
    "svelte": "^5.0.0",
    "typescript": "^5.5.0",
    "vite": "^6.0.0"
  }
}
```

- [ ] **Step 2: 创建 index.html**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Trove2</title>
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 3: 创建 svelte.config.js**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
};
```

- [ ] **Step 4: 创建 tsconfig.json**

```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleDetection": "force"
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 5: 创建 tsconfig.node.json**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 6: 创建 vite.config.ts**

```ts
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 5174 }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
```

- [ ] **Step 7: 创建 .gitignore**

```
node_modules/
dist/
.DS_Store
src-tauri/target/
src-tauri/gen/
```

- [ ] **Step 8: 设置代理并安装 npm 依赖**

```bash
# 设置代理（中国大陆网络环境）
export http_proxy=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
# 安装依赖
npm install
# 验证 Vite 能正常工作
npm run build
```

验证：`npm run build` 应输出 `dist/` 目录，无报错。

---

### Task 2: Tauri Rust 后端 — 基础结构

**文件：**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

**说明：** 搭建 Tauri v2 的 Rust 后端基础骨架，此时不包含任何业务代码，验证 cargo 能编译通过。

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "trove2"
version = "0.1.0"
description = "Personal tool collection app"
authors = ["ripple"]
edition = "2021"

[lib]
name = "trove2_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: 创建 build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: 创建 src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    trove2_lib::run()
}
```

- [ ] **Step 4: 创建 src-tauri/src/lib.rs**

```rust
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 创建 src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/schema.json",
  "productName": "Trove2",
  "version": "0.1.0",
  "identifier": "com.trove2.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "Trove2",
        "width": 1000,
        "height": 700,
        "minWidth": 700,
        "minHeight": 500
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 6: 创建 src-tauri/capabilities/default.json**

```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 7: 验证 Rust 编译**

```bash
export http_proxy=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
cd src-tauri && cargo check
```

验证：`cargo check` 无报错。

---

### Task 3: Rust 后端 — Tool 模型 + get_tools 命令

**文件：**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/tool.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/tools.rs`
- Modify: `src-tauri/src/lib.rs`

**说明：** 定义 Tool 数据模型，实现 `get_tools` IPC 命令，空壳阶段返回空列表。

- [ ] **Step 1: 创建 models/mod.rs**

```rust
pub mod tool;
```

- [ ] **Step 2: 创建 models/tool.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: String,
}
```

- [ ] **Step 3: 创建 commands/mod.rs**

```rust
pub mod tools;
```

- [ ] **Step 4: 创建 commands/tools.rs**

```rust
use crate::models::tool::Tool;

#[tauri::command]
pub fn get_tools() -> Vec<Tool> {
    vec![]
}
```

- [ ] **Step 5: 修改 lib.rs，注册 models、commands 模块和 IPC 命令**

```rust
mod commands;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::tools::get_tools])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: 验证编译**

```bash
export http_proxy=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
cd src-tauri && cargo check
```

验证：无报错。

---

### Task 4: Svelte 入口 + 毛玻璃主题样式

**文件：**
- Create: `src/main.ts`
- Create: `src/vite-env.d.ts`
- Create: `src/app.css`
- Create: `src/App.svelte`（基础框架，不含路由逻辑）

**说明：** 建立 Svelte 挂载入口和全局毛玻璃主题 CSS。

- [ ] **Step 1: 创建 src/vite-env.d.ts**

```ts
/// <reference types="svelte" />
/// <reference types="vite/client" />
```

- [ ] **Step 2: 创建 src/main.ts**

```ts
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

const app = mount(App, { target: document.getElementById('app')! });

export default app;
```

- [ ] **Step 3: 创建 src/app.css（毛玻璃主题）**

```css
:root {
  --glass-bg: rgba(255, 255, 255, 0.08);
  --glass-border: rgba(255, 255, 255, 0.12);
  --glass-hover: rgba(255, 255, 255, 0.14);
  --text-primary: rgba(255, 255, 255, 0.92);
  --text-secondary: rgba(255, 255, 255, 0.6);
  --text-muted: rgba(255, 255, 255, 0.38);
  --accent: #7c6cf0;
  --accent-hover: #9184f5;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --shadow-glass: 0 4px 24px rgba(0, 0, 0, 0.3);
  --transition: 0.2s ease;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC',
    'Hiragino Sans GB', 'Microsoft YaHei', sans-serif;
  background: linear-gradient(135deg, #0f0c29, #302b63, #24243e);
  color: var(--text-primary);
  min-height: 100vh;
  overflow-x: hidden;
  -webkit-font-smoothing: antialiased;
}

#app {
  min-height: 100vh;
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
}

::selection {
  background: var(--accent);
  color: white;
}
```

- [ ] **Step 4: 创建临时 App.svelte（纯测试，后续会改）**

```svelte
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
```

- [ ] **Step 5: 验证 Svelte + Tauri 联动**

```bash
# 先构建前端
npm run build
# 检查 Tauri 能识别
npx tauri info
```

---

### Task 5: 前端基础设施 — 类型 / Store / 路由

**文件：**
- Create: `src/lib/types.ts`
- Create: `src/lib/stores.ts`
- Create: `src/lib/router.ts`

**说明：** 建立前端共享的数据类型、状态管理和路由工具函数，不涉及 UI。

- [ ] **Step 1: 创建 src/lib/types.ts**

```ts
export interface Tool {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string;
}
```

- [ ] **Step 2: 创建 src/lib/stores.ts**

```ts
import { writable } from 'svelte/store';
import type { Tool } from './types';

export const tools = writable<Tool[]>([]);
export const searchQuery = writable('');
export const selectedCategory = writable('');
```

- [ ] **Step 3: 创建 src/lib/router.ts**

```ts
import { writable } from 'svelte/store';

export interface Route {
  page: 'home' | 'tool' | 'notfound';
  params: Record<string, string>;
}

function parseHash(): Route {
  const hash = window.location.hash.slice(1) || '/';

  if (hash === '/') {
    return { page: 'home', params: {} };
  }

  const match = hash.match(/^\/tool\/(.+)$/);
  if (match) {
    return { page: 'tool', params: { id: decodeURIComponent(match[1]) } };
  }

  return { page: 'notfound', params: {} };
}

export const currentRoute = writable<Route>(parseHash());

export function navigateTo(path: string) {
  window.location.hash = path;
}

if (typeof window !== 'undefined') {
  window.addEventListener('hashchange', () => {
    currentRoute.set(parseHash());
  });
}
```

---

### Task 6: 可复用毛玻璃 UI 组件

**文件：**
- Create: `src/lib/components/GlassPanel.svelte`
- Create: `src/lib/components/ToolCard.svelte`
- Create: `src/lib/components/NavBar.svelte`
- Create: `src/lib/components/EmptyState.svelte`

**说明：** 构建所有可复用的毛玻璃风格 UI 组件。

- [ ] **Step 1: 创建 GlassPanel.svelte**

```svelte
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
```

- [ ] **Step 2: 创建 ToolCard.svelte**

```svelte
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
```

- [ ] **Step 3: 创建 NavBar.svelte**

```svelte
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
    <h1 class="nav-title" onclick={goHome} role="button" tabindex="0">Trove2</h1>
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
```

- [ ] **Step 4: 创建 EmptyState.svelte**

```svelte
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
```

---

### Task 7: 主页 + 工具容器页 + App 路由集成

**文件：**
- Create: `src/lib/pages/Home.svelte`
- Create: `src/lib/pages/ToolView.svelte`
- Modify: `src/App.svelte`

**说明：** 组装完整的应用页面，实现搜索/筛选/空状态/工具页面切换。

- [ ] **Step 1: 创建 Home.svelte**

```svelte
<script lang="ts">
  import { tools, searchQuery, selectedCategory } from '../stores';
  import ToolCard from '../components/ToolCard.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import type { Tool } from '../types';

  let allTools = $state<Tool[]>([]);
  let query = $state('');
  let category = $state('');

  const CATEGORIES = ['全部', '文本', '转换', '加密', '开发', '图片', '其他'];

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
        {#each CATEGORIES as cat}
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
```

- [ ] **Step 2: 创建 ToolView.svelte**

```svelte
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
```

- [ ] **Step 3: 修改 App.svelte（完整路由集成）**

```svelte
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
```

---

### Task 8: 构建验证

**说明：** 验证整个应用可以编译打包并预览。

- [ ] **Step 1: 构建前端**

```bash
export http_proxy=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
npm run build
```

期望输出：`dist/` 目录生成，无报错。

- [ ] **Step 2: 构建 Tauri 应用**

```bash
export http_proxy=http://127.0.0.1:7897
export https_proxy=http://127.0.0.1:7897
npx tauri build
```

期望输出：编译成功，生成可执行文件（`src-tauri/target/release/trove2`）。

- [ ] **Step 3: 页面状态速查表**

| 状态 | 触发条件 | 表现 |
|------|---------|------|
| 空主页 | `get_tools` 返回 `[]` | 🔧 "还没有工具，敬请期待" |
| 工具页面 | 路由 `#/tool/test`（ID 不存在） | ❓ "工具未找到" + 返回按钮 |
| 404 页面 | 路由 `#/anything` | 🌐 "页面不存在" + 返回链接 |
| 搜索无结果 | 输入不存在的工具名 | 🔍 "没有匹配的工具" |
