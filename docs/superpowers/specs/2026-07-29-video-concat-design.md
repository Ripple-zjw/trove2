# 视频拼接工具 (Video Concat) 设计文档

## 概述

为 Trove2 添加一个"视频拼接"工具，让用户选取多个本地视频文件，排序后通过系统 ffmpeg 合并成一个完整视频。

## 技术栈

- **视频处理**: 调用系统 `ffmpeg`（`std::process::Command`）
- **文件选择**: `tauri-plugin-dialog` (系统原生对话框)
- **格式兼容**: 混合策略 — 先尝试无损 concat demuxer + `-c copy`，失败自动回退到 `-filter_complex concat`（重编码）
- **进度推送**: Tauri Events（Rust `emit` → Svelte 订阅）

## 架构

```
Frontend (Svelte 5)                   Backend (Rust)
┌──────────────────────────┐  invoke  ┌─────────────────────┐
│ VideoConcat.svelte        │ ←─────── │ commands/video.rs   │
│  ┌──────────────────┐    │  events  │                     │
│  │ 状态区 (ffmpeg信息)│    │         │ check_ffmpeg()      │
│  │ 操作区 (文件+选项) │    │         │ concat_videos()     │
│  │ 执行区 (进度+结果) │    │  ─────→ │  ffmpeg进程管理     │
│  └──────────────────┘    │         │  进度解析+emit       │
└──────────────────────────┘         └─────────────────────┘
```

## Rust 后端设计

### 新增文件: `src/commands/video.rs`

```rust
#[derive(Serialize)]
pub struct FfmpegInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ConcatProgress {
    pub percent: f64,       // 0.0 ~ 100.0
    pub current_file: String,
    pub speed: String,      // e.g. "3.45x"
    pub eta: String,
}

#[derive(Serialize)]
pub struct ConcatResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub file_size: Option<u64>,
}
```

### `check_ffmpeg()`
- `which ffmpeg` (Unix) / `where ffmpeg` (Windows)
- `ffmpeg -version` → 解析第一行版本号
- 返回 FfmpegInfo

### `concat_videos(inputs: Vec<String>, output_path: String, format: String)`
- 先 `ffprobe` 检查所有输入文件编码是否一致（判断能否 -c copy）
- 策略:
  1. 所有文件同编码 → `-f concat -safe 0 -i list.txt -c copy`
  2. 编码不一致 → 直接 `-filter_complex concat=n=N:v=1:a=1`（重编码）
- 总时长: 用 `ffprobe -v quiet -show_entries format=duration -of csv=p=0` 对每个文件计算后求和
- 通过 `app_handle.emit("concat-progress", payload)` 推送进度
- 支持取消: 前端发消息 → Rust 设置 AtomicBool → ffmpeg 子进程定期检查 → kill

### 异常处理
- ffmpeg 未安装 → 前端直接显示错误，不允许拼接
- 文件不存在 → 返回具体文件名
- ffmpeg 进程异常退出 → 解析 stderr 返回用户可读的信息
- 输出路径无写入权限 → 提示检查权限
- 所有文件同格式但流参数不兼容（如不同分辨率/帧率）→ `-c copy` 失败自动回退

## 前端设计

### 新增组件: `src/lib/components/VideoConcat.svelte`

三个区域的状态机：

| 状态 | 展示 |
|------|------|
| **未检测ffmpeg** | 加载中动画 |
| **已安装** | 绿色 ✔ 显示版本 + 路径 |
| **未安装** | 红色 ❌ 错误信息 + 如何安装指引 |
| **未选文件** | 「选择视频」按钮 + 提示 |
| **已选文件** | 文件列表（可拖拽排序）+ 格式选择 + 输出路径 |
| **拼接中** | 进度条 + 速度 + ETA + 取消按钮 |
| **完成** | 成功/失败信息 + 打开文件夹按钮 + 继续拼接 |

### 交互细节
- 拖拽排序: 原生 HTML5 Drag & Drop（无额外依赖）
- 移除单个文件: 每个文件卡片右上角 ✕ 按钮
- 清空列表: 「清空」按钮
- 文件格式过滤: `input.accept = "video/mp4,video/quicktime,video/x-msvideo,video/x-matroska,video/webm,video/mpeg"` — 但用户仍可选择所有文件（防止格式误判）
- 输出格式下拉: MP4 / MKV / MOV / AVI / WebM（对应容器扩展名）
- 输出路径: 默认首个输入文件同级目录，文件名 = `concat_{时间戳}`，用户可自选

### 边界状态
- 仅选 1 个文件 → 按钮禁用 + 提示"至少选择 2 个视频"
- 文件中有非视频 → ffprobe 验证，跳过无效
- 拼接中途取消 → 清理未完成的临时输出文件
- 重名 → 自动追加序号

## 数据流

```
点击「开始拼接」
  → invoke('concat_videos', { inputs, output, format })
  → Rust 启动 ffprobe 获取总时长
  → 传回前端（初始化进度）
  → Rust 启动 ffmpeg
  → 循环: ffmpeg 输出 → 解析 → emit progress event
  → Svelte listen → 更新进度条
  → 完成: emit result event
  → 前端显示"拼接完成"
```

## 文件变更清单

| 文件 | 操作 |
|------|------|
| `src-tauri/Cargo.toml` | 新增 `tauri-plugin-dialog` |
| `src-tauri/src/lib.rs` | 注册 `video` 模块命令 + 插件 |
| `src-tauri/src/commands/mod.rs` | 添加 `pub mod video;` |
| `src-tauri/src/commands/video.rs` | **新建** — 核心逻辑 |
| `src-tauri/capabilities/default.json` | 允许 dialog 和 event 权限 |
| `src/lib/components/VideoConcat.svelte` | **新建** — 工具 UI |
| `src/App.svelte` | 注册 VideoConcat 映射 |
| `src/lib/stores.ts` | 可能添加状态支持 |

## 不做的事情 (YAGNI)

- 不内置 ffmpeg 二进制
- 不提供视频裁剪/截取功能（仅拼接）
- 不提供视频编码参数细调（交给 ffmpeg 默认）
- 不提供预览功能
- 不维护历史记录
