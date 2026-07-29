# Trove2 — 个人工具集合应用 设计文档

## 概述

基于 Rust + Tauri v2 + Svelte 的个人桌面工具集合应用。一个毛玻璃风格的主页，展示所有可用工具，点击进入对应工具页面。先从空壳开始，逐步添加工具。

## 技术栈

- **桌面框架**: Tauri v2
- **前端**: Svelte（编译型，无运行时开销）
- **路由**: 原生 hash 路由，无额外依赖
- **样式**: CSS 毛玻璃（`backdrop-filter: blur`），暗色主题，跟随系统深色模式
- **构建**: Vite + Svelte 插件

## 架构

```
Tauri v2 WebView
├── Svelte SPA (前端)
│   ├── App.svelte (根组件 + 路由出口)
│   ├── Home.svelte (主页: 搜索/分类/卡片网格)
│   ├── ToolView.svelte (工具容器: 动态渲染)
│   └── 组件: GlassPanel / ToolCard / NavBar / EmptyState
├── Tauri IPC (invoke)
└── Rust 后端
    ├── models/tool.rs — Tool 结构体 (id, name, desc, category, icon)
    └── commands/tools.rs — get_tools() IPC 命令
```

## 页面 & 路由

| 路由 | 页面 | 说明 |
|------|------|------|
| `#/` | Home | 搜索框 + 分类筛选 + 工具卡片网格 |
| `#/tool/:id` | ToolView | 根据工具 ID 渲染组件；无效 ID → "未找到"提示 |

## 数据流

1. 应用启动 → `invoke('get_tools')` → Rust 返回硬编码工具列表 → 前端 store 缓存
2. 搜索/筛选 → 纯前端过滤（Svelte reactive）
3. 点击卡片 → hash 路由切换 → ToolView 根据 ID 加载对应组件

## 边界状态

- **工具列表为空**: 主页显示空状态（"还没有工具" + 毛玻璃占位卡片）
- **搜索无结果**: "没有匹配的工具"
- **无效工具 ID**: "工具不存在" + 返回主页按钮

## 毛玻璃风格

- 背景: 深色渐变（CSS `linear-gradient`）
- 卡片: `background: rgba(255,255,255,0.08); backdrop-filter: blur(20px); border: 1px solid rgba(255,255,255,0.12); border-radius: 16px;`
- 文字: 白色/浅色，高对比度

## 开始顺序

1. 脚手架: 初始化 Svelte + Tauri v2 项目
2. Rust 后端: Tool 模型 + get_tools 命令（返回空数组）
3. 前端路由 + 毛玻璃主题
4. 主页: 搜索框 + 分类筛选 + 空状态
5. 工具容器页 + 未找到处理
6. 验证构建 & 运行
