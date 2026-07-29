# 视频拼接工具 (Video Concat) 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标:** 为 Trove2 添加一个视频拼接工具，用户选取多个视频文件，排序后通过系统 ffmpeg 合并成一个视频。

**架构:** Rust 后端通过 `std::process::Command` 调用系统 ffmpeg，用 Tauri Events 推送进度。前端 Svelte 5 组件提供文件列表（拖拽排序）、格式选择、进度展示。

**技术栈:** Tauri v2, Svelte 5, Rust, ffmpeg（系统调用）, `tauri-plugin-dialog`（文件/目录选择器）

## 全局约束

- Rust 端使用 `std::process::Command` 调用系统 ffmpeg，不内嵌 ffmpeg 二进制
- `tauri-plugin-dialog` Rust crate v2 + `@tauri-apps/plugin-dialog` JS 包
- 毛玻璃 UI 风格对齐现有 CSS 变量（`--glass-bg`, `--accent` 等）
- 所有输出文本使用中文
- 遵守工作规范：不允许 git worktree，在当前 branch 上直接修改

---

### Task 1: Rust 后端 — check_ffmpeg + 工具注册

**文件:**
- Create: `src-tauri/src/commands/video.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/commands/tools.rs`

**说明:** 实现 ffmpeg 检测命令，并将视频拼接工具注册到工具列表。

- [ ] **Step 1: 在 Cargo.toml 中添加 `tauri-plugin-dialog` 依赖**

```toml
[dependencies]
# ... existing deps
tauri-plugin-dialog = "2"
```

- [ ] **Step 2: 创建 `commands/video.rs`，实现 `check_ffmpeg`**

```rust
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize, Clone)]
pub struct FfmpegInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConcatProgress {
    pub percent: f64,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConcatResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub file_size: Option<u64>,
}

#[tauri::command]
pub fn check_ffmpeg() -> FfmpegInfo {
    // 查找 ffmpeg 路径
    let path = which_ffmpeg();
    
    if let Some(path_str) = &path {
        let output = Command::new("ffmpeg")
            .arg("-version")
            .output();
        
        match output {
            Ok(out) if out.status.success() => {
                let version_str = String::from_utf8_lossy(&out.stdout);
                let version_line = version_str.lines().next().unwrap_or("").to_string();
                FfmpegInfo {
                    installed: true,
                    version: Some(version_line),
                    path: Some(path_str.clone()),
                }
            }
            _ => FfmpegInfo {
                installed: false,
                version: None,
                path: None,
            },
        }
    } else {
        FfmpegInfo {
            installed: false,
            version: None,
            path: None,
        }
    }
}

fn which_ffmpeg() -> Option<String> {
    let which_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    Command::new(which_cmd)
        .arg("ffmpeg")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}
```

- [ ] **Step 3: 更新 `commands/mod.rs`**

```rust
pub mod tools;
pub mod video;
```

- [ ] **Step 4: 更新 `tools.rs`，在 `get_tools()` 中返回视频拼接工具**

```rust
use crate::models::tool::Tool;

#[tauri::command]
pub fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            id: "video-concat".into(),
            name: "视频拼接".into(),
            description: "将多个视频文件首尾相连合并成一个视频。支持 MP4、MOV、MKV、AVI 等常见格式。".into(),
            category: "转换".into(),
            icon: "🎬".into(),
        },
    ]
}
```

- [ ] **Step 5: 更新 `lib.rs`，注册 dialog 插件和 video 命令**

```rust
mod commands;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::tools::get_tools,
            commands::video::check_ffmpeg,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: 验证编译**

```bash
cd /Users/ripple/projects/trove2 && cargo build -p trove2
```

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: add check_ffmpeg command and register video-concat tool"
```

---

### Task 2: Rust 后端 — concat_videos 核心逻辑

**文件:**
- Create: `src-tauri/src/commands/video.rs`（追加到 Task 1 的文件中）
- Modify: `src-tauri/src/lib.rs`（注册新命令 + 状态管理）

**说明:** 实现视频拼接核心逻辑：ffprobe 获取编码信息 → 选择策略 → 执行 ffmpeg → 解析进度 → 事件推送 → 支持取消。

- [ ] **Step 1: 在 `lib.rs` 中添加 AppState（取消标志）**

```rust
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::tools::get_tools,
            commands::video::check_ffmpeg,
            commands::video::concat_videos,
            commands::video::cancel_concat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 在 `video.rs` 中添加 ffprobe 辅助函数**

```rust
use std::path::Path;

/// 检查所有输入视频的编码是否一致（决定是否能用 -c copy）
fn check_all_same_codec(inputs: &[String]) -> Result<bool, String> {
    let mut first_codec: Option<String> = None;
    
    for input in inputs {
        let output = Command::new("ffprobe")
            .args(["-v", "quiet", "-select_streams", "v:0", "-show_entries", "stream=codec_name", "-of", "csv=p=0", input])
            .output()
            .map_err(|e| format!("无法执行 ffprobe: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("ffprobe 分析失败: {}", input));
        }
        
        let codec = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if codec.is_empty() {
            return Err(format!("无法获取视频编码信息: {}", input));
        }
        
        match &first_codec {
            None => first_codec = Some(codec),
            Some(c) if *c != codec => return Ok(false),
            _ => {}
        }
    }
    Ok(true)
}

/// 获取所有输入文件的总时长（微秒）
fn get_total_duration_us(inputs: &[String]) -> Result<u64, String> {
    let mut total: f64 = 0.0;
    
    for input in inputs {
        let output = Command::new("ffprobe")
            .args(["-v", "quiet", "-show_entries", "format=duration", "-of", "csv=p=0", input])
            .output()
            .map_err(|e| format!("无法执行 ffprobe: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("无法获取视频时长: {}", input));
        }
        
        let dur_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let dur: f64 = dur_str.parse().map_err(|_| "无法解析时长".to_string())?;
        total += dur;
    }
    
    Ok((total * 1_000_000.0) as u64)
}
```

- [ ] **Step 3: 在 `video.rs` 中添加 `concat_videos` 命令**

```rust
use tauri::Emitter;
use std::sync::atomic::Ordering;
use std::io::BufRead;
use crate::AppState;

#[tauri::command]
pub async fn concat_videos(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output_path: String,
    format: String,
) -> Result<ConcatResult, String> {
    let start = std::time::Instant::now();
    
    // 重置取消标志
    app.state::<AppState>().cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel_flag = app.state::<AppState>().cancel_flag.clone();
    
    // 获取总时长
    let total_us = get_total_duration_us(&inputs)?;
    
    // 检查编码一致性
    let same_codec = check_all_same_codec(&inputs)?;
    
    // 创建临时文件列表（concat demuxer 需要）
    let list_path = if same_codec {
        let list_content: String = inputs.iter()
            .map(|p| format!("file '{}'", p.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join("\n");
        let list_file = std::env::temp_dir().join(format!("concat_list_{}.txt", std::process::id()));
        std::fs::write(&list_file, &list_content).ok();
        Some(list_file)
    } else {
        None
    };
    
    // 构建 ffmpeg 命令
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y"); // 覆盖输出
    
    if same_codec {
        // 策略 1: concat demuxer + copy（无损，快速）
        cmd.args(["-f", "concat", "-safe", "0", "-i"])
            .arg(list_path.as_ref().unwrap().to_string_lossy().as_ref())
            .args(["-c", "copy"]);
    } else {
        // 策略 2: filter_complex concat（重编码，兼容不同格式）
        let n = inputs.len();
        for input in &inputs {
            cmd.args(["-i", input]);
        }
        let filter = format!("[0:v][0:a]{}", (1..n).flat_map(|i| vec![format!("[{i}:v]"), format!("[{i}:a]")]).collect::<String>());
        let filter = format!("{}concat=n={}:v=1:a=1[v][a]", filter, n);
        cmd.args(["-filter_complex", &filter])
            .args(["-map", "[v]", "-map", "[a]"]);
    }
    
    // 输出格式映射
    let container = match format.as_str() {
        "mkv" => "mkv",
        "mov" => "mov",
        "avi" => "avi",
        "webm" => "webm",
        _ => "mp4",
    };
    let out_with_ext = if output_path.ends_with(&format!(".{container}")) {
        output_path.clone()
    } else {
        format!("{}.{}", output_path.trim_end_matches(&['.', ' '][..]), container)
    };
    cmd.arg(&out_with_ext);
    
    // 启用进度输出
    cmd.arg("-progress").arg("pipe:1");
    cmd.arg("-nostats");
    
    // 启动 ffmpeg
    let mut child = cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 ffmpeg: {}（请检查 ffmpeg 是否已安装）", e))?;
    
    let stdout = child.stdout.take().unwrap();
    let reader = std::io::BufReader::new(stdout);
    
    // 解析进度行
    let mut out_time_us: u64 = 0;
    
    for line in reader.lines() {
        // 检查取消标志
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = std::fs::remove_file(&out_with_ext);
            return Err("用户取消了拼接".into());
        }
        
        match line {
            Ok(l) if l.starts_with("out_time_us=") => {
                if let Ok(val) = l.trim_start_matches("out_time_us=").parse::<u64>() {
                    out_time_us = val;
                    let percent = if total_us > 0 {
                        (out_time_us as f64 / total_us as f64 * 100.0).min(99.9)
                    } else {
                        0.0
                    };
                    
                    let elapsed = start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        format!("{:.2}x", out_time_us as f64 / (elapsed * 1_000_000.0))
                    } else {
                        "0.00x".into()
                    };
                    
                    let remaining = if percent > 0.0 {
                        let total_secs = elapsed * 100.0 / percent;
                        let eta_secs = (total_secs - elapsed) as u64;
                        format!("{}:{:02}", eta_secs / 60, eta_secs % 60)
                    } else {
                        "--:--".into()
                    };
                    
                    let _ = app.emit("concat-progress", ConcatProgress {
                        percent,
                        speed,
                        eta: remaining,
                    });
                }
            }
            Ok(l) if l == "progress=end" => break,
            _ => {}
        }
    }
    
    let status = child.wait().map_err(|e| format!("等待 ffmpeg 进程失败: {}", e))?;
    
    // 清理临时文件
    if let Some(list_path) = list_path {
        let _ = std::fs::remove_file(list_path);
    }
    
    if !status.success() {
        // 如果 -c copy 失败，且还未重试，尝试回退
        if same_codec {
            // 读取 stderr 获取错误信息
            let stderr = child.stderr.take()
                .map(|s| {
                    let mut buf = String::new();
                    std::io::BufReader::new(s).read_to_string(&mut buf).ok();
                    buf
                })
                .unwrap_or_default();
            
            // 如果是因为编码不兼容导致，自动回退到 filter_complex
            // 这里简化处理：将 same_codec 设为 false 重试
            if stderr.contains("Non-monotonous DTS") || stderr.contains("Invalid") {
                drop(child);
                let _ = std::fs::remove_file(&out_with_ext);
                // 递归调用自身进行重编码策略
                return concat_videos_with_encoding(app, inputs, output_path, format).await;
            }
        }
        
        return Err(format!("ffmpeg 拼接失败（退出码: {}）", status.code().unwrap_or(-1)));
    }
    
    let file_size = std::fs::metadata(&out_with_ext).ok().map(|m| m.len());
    let elapsed_ms = start.elapsed().as_millis() as u64;
    
    Ok(ConcatResult {
        success: true,
        output_path: Some(out_with_ext),
        error: None,
        duration_ms: elapsed_ms,
        file_size,
    })
}
```

- [ ] **Step 4: 添加 `cancel_concat` 命令**

```rust
#[tauri::command]
pub fn cancel_concat(state: tauri::State<'_, AppState>) {
    state.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
}
```

- [ ] **Step 5: 编译验证**

```bash
cd /Users/ripple/projects/trove2 && cargo build -p trove2
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: add concat_videos core logic with progress and cancel"
```

---

### Task 3: 前端 — 安装 dialog 插件 + VideoConcat 组件

**文件:**
- Modify: `package.json`
- Create: `src/lib/components/VideoConcat.svelte`

**说明:** 安装前端 dialog 依赖，实现完整的视频拼接 UI 组件。

- [ ] **Step 1: 安装 `@tauri-apps/plugin-dialog`**

```bash
cd /Users/ripple/projects/trove2 && npm install @tauri-apps/plugin-dialog
```

- [ ] **Step 2: 创建 `VideoConcat.svelte` 组件**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import GlassPanel from './GlassPanel.svelte';

  // --- 状态 ---
  let ffmpegInfo = $state<{ installed: boolean; version?: string; path?: string } | null>(null);
  let ffmpegLoading = $state(true);

  let files = $state<string[]>([]);
  let outputFormat = $state('mp4');
  let outputPath = $state('');
  let dragIndex = $state<number | null>(null);

  let running = $state(false);
  let progress = $state<{ percent: number; speed: string; eta: string } | null>(null);
  let result = $state<{ success: boolean; output_path?: string; error?: string; duration_ms?: number; file_size?: number } | null>(null);

  const formats = [
    { value: 'mp4', label: 'MP4 (H.264)', ext: '.mp4' },
    { value: 'mkv', label: 'MKV', ext: '.mkv' },
    { value: 'mov', label: 'MOV', ext: '.mov' },
    { value: 'avi', label: 'AVI', ext: '.avi' },
    { value: 'webm', label: 'WebM', ext: '.webm' },
  ];
  let selectedFormat = $derived(formats.find(f => f.value === outputFormat)!);

  // --- 生命周期 ---
  $effect(() => {
    checkFfmpeg();
  });

  async function checkFfmpeg() {
    ffmpegLoading = true;
    try {
      ffmpegInfo = await invoke('check_ffmpeg');
    } catch (e) {
      ffmpegInfo = { installed: false };
    }
    ffmpegLoading = false;
  }

  // --- 文件选择 ---
  async function selectFiles() {
    const selected = await open({
      multiple: true,
      filters: [{
        name: '视频文件',
        extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'mpeg', 'wmv', 'flv'],
      }],
    });
    if (selected) {
      files = [...files, ...selected];
    }
  }

  function removeFile(index: number) {
    files = files.filter((_, i) => i !== index);
  }

  function clearFiles() {
    files = [];
    result = null;
  }

  // --- 拖拽排序 ---
  function handleDragStart(e: DragEvent, index: number) {
    dragIndex = index;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
    }
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === index) return;
    const newFiles = [...files];
    const [moved] = newFiles.splice(dragIndex, 1);
    newFiles.splice(index, 0, moved);
    files = newFiles;
    dragIndex = index;
  }

  function handleDragEnd() {
    dragIndex = null;
  }

  // --- 输出路径 ---
  async function selectOutputPath() {
    const selected = await save({
      filters: [{
        name: '视频文件',
        extensions: [outputFormat],
      }],
      defaultPath: `concat_${Date.now()}.${outputFormat}`,
    });
    if (selected) {
      outputPath = selected;
    }
  }

  function formatFileSize(bytes?: number): string {
    if (!bytes) return '--';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function formatDuration(ms?: number): string {
    if (!ms) return '--';
    if (ms < 1000) return `${ms} 毫秒`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)} 秒`;
    const m = Math.floor(ms / 60000);
    const s = Math.round((ms % 60000) / 1000);
    return `${m} 分 ${s} 秒`;
  }

  function getFileName(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1] || path;
  }

  // --- 拼接 ---
  async function startConcat() {
    if (files.length < 2) return;
    result = null;
    running = true;
    progress = null;

    const unlisten = await listen<{ percent: number; speed: string; eta: string }>('concat-progress', (event) => {
      progress = event.payload;
    });

    try {
      const res = await invoke('concat_videos', {
        inputs: files,
        outputPath: outputPath || `concat_${Date.now()}.${outputFormat}`,
        format: outputFormat,
      });
      result = res as any;
    } catch (e) {
      result = { success: false, error: String(e) };
    } finally {
      running = false;
      unlisten();
    }
  }

  async function cancelConcat() {
    await invoke('cancel_concat');
  }

  async function openOutputFolder() {
    if (result?.output_path) {
      await invoke('show_item_in_folder', { path: result.output_path });
    }
  }

  let canStart = $derived.by(() => {
    return !running && ffmpegInfo?.installed && files.length >= 2;
  });

  let validationMsg = $derived.by(() => {
    if (files.length < 2 && files.length > 0) return '至少选择 2 个视频文件';
    return '';
  });
</script>

<div class="video-concat">
  <!-- ① ffmpeg 状态 -->
  <GlassPanel padding="16px" class="status-panel">
    {#if ffmpegLoading}
      <p class="status-loading">正在检测 ffmpeg…</p>
    {:else if ffmpegInfo?.installed}
      <div class="status-ok">
        <span class="status-dot dot-ok"></span>
        <div>
          <p class="status-title">ffmpeg 已就绪</p>
          <p class="status-detail">{ffmpegInfo.version?.split(/[,\r\n]/)[0]}</p>
          <p class="status-detail">{ffmpegInfo.path}</p>
        </div>
      </div>
    {:else}
      <div class="status-err">
        <span class="status-dot dot-err"></span>
        <div>
          <p class="status-title">未检测到 ffmpeg</p>
          <p class="status-detail">
            请安装 ffmpeg：<code>brew install ffmpeg</code>（macOS）或从官网下载
          </p>
        </div>
      </div>
    {/if}
  </GlassPanel>

  <!-- ② 操作区 -->
  <GlassPanel padding="20px" class="action-panel">
    <div class="action-header">
      <h3>选择视频文件</h3>
      {#if files.length > 0}
        <button class="btn-text" onclick={clearFiles}>清空</button>
      {/if}
    </div>

    {#if files.length === 0}
      <div class="file-empty" onclick={selectFiles} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && selectFiles()}>
        <span class="file-empty-icon">📁</span>
        <p>点击选择视频文件</p>
        <p class="file-empty-hint">支持 MP4、MOV、MKV、AVI、WebM 等格式</p>
      </div>
    {:else}
      <div class="file-list">
        {#each files as file, i (file)}
          <div
            class="file-item"
            draggable="true"
            ondragstart={(e) => handleDragStart(e, i)}
            ondragover={(e) => handleDragOver(e, i)}
            ondragend={handleDragEnd}
            class:dragging={dragIndex === i}
          >
            <span class="drag-handle">⠿</span>
            <span class="file-index">{i + 1}</span>
            <span class="file-name">{getFileName(file)}</span>
            <button class="btn-remove" onclick={() => removeFile(i)} title="移除">✕</button>
          </div>
        {/each}
        <button class="btn-add-more" onclick={selectFiles}>+ 添加更多文件</button>
      </div>
    {/if}

    <div class="options-row">
      <div class="option-group">
        <label>输出格式</label>
        <select bind:value={outputFormat} class="select-glass">
          {#each formats as fmt}
            <option value={fmt.value}>{fmt.label}</option>
          {/each}
        </select>
      </div>
      <div class="option-group">
        <label>输出路径</label>
        <div class="path-row">
          <input
            type="text"
            bind:value={outputPath}
            placeholder="可选，默认在首个文件同目录"
            class="input-glass"
            readonly
          />
          <button class="btn-secondary" onclick={selectOutputPath}>选择</button>
        </div>
      </div>
    </div>

    {#if validationMsg}
      <p class="validation-msg">{validationMsg}</p>
    {/if}
  </GlassPanel>

  <!-- ③ 执行区 -->
  <GlassPanel padding="20px" class="exec-panel">
    {#if running}
      <div class="progress-section">
        <div class="progress-bar">
          <div class="progress-fill" style="width: {progress?.percent ?? 0}%"></div>
        </div>
        <div class="progress-info">
          <span>{(progress?.percent ?? 0).toFixed(1)}%</span>
          <span>速度: {progress?.speed ?? '--'}</span>
          <span>剩余: {progress?.eta ?? '--'}</span>
        </div>
        <button class="btn-cancel" onclick={cancelConcat}>取消拼接</button>
      </div>
    {:else if result}
      {#if result.success}
        <div class="result-success">
          <span class="result-icon">✅</span>
          <div>
            <p class="result-title">拼接完成！</p>
            <p class="result-detail">
              大小: {formatFileSize(result.file_size)} | 耗时: {formatDuration(result.duration_ms)}
            </p>
            {#if result.output_path}
              <p class="result-path">📄 {getFileName(result.output_path)}</p>
            {/if}
            <div class="result-actions">
              <button class="btn-primary" onclick={openOutputFolder}>📂 打开所在文件夹</button>
              <button class="btn-secondary" onclick={clearFiles}>继续拼接</button>
            </div>
          </div>
        </div>
      {:else}
        <div class="result-error">
          <span class="result-icon">❌</span>
          <div>
            <p class="result-title">拼接失败</p>
            <p class="result-detail">{result.error || '未知错误'}</p>
          </div>
        </div>
      {/if}
    {:else}
      <div class="exec-placeholder">
        <p>选择至少 2 个视频文件，调整顺序后开始拼接</p>
      </div>
    {/if}

    {#if !running && !result}
      <button
        class="btn-start"
        disabled={!canStart}
        onclick={startConcat}
      >
        🚀 开始拼接
      </button>
    {/if}
  </GlassPanel>
</div>

<style>
  .video-concat {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* --- ffmpeg 状态 --- */
  .status-panel { font-size: 14px; }
  .status-loading { color: var(--text-muted); }
  .status-ok, .status-err { display: flex; align-items: flex-start; gap: 12px; }
  .status-dot {
    width: 10px; height: 10px; border-radius: 50%;
    flex-shrink: 0; margin-top: 4px;
  }
  .dot-ok { background: #4ade80; }
  .dot-err { background: #f87171; }
  .status-title { font-weight: 600; margin-bottom: 2px; }
  .status-detail { color: var(--text-secondary); font-size: 12px; margin-top: 1px; }
  .status-detail code {
    background: rgba(255,255,255,0.1); padding: 1px 6px;
    border-radius: 4px; font-size: 12px;
  }

  /* --- 操作区 --- */
  .action-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 12px;
  }
  .action-header h3 { font-size: 16px; }
  .btn-text {
    background: none; border: none; color: var(--accent);
    cursor: pointer; font-size: 13px;
  }
  .btn-text:hover { color: var(--accent-hover); }

  /* 空文件状态 */
  .file-empty {
    border: 2px dashed var(--glass-border); border-radius: var(--radius-md);
    padding: 40px 20px; text-align: center; cursor: pointer;
    transition: border-color var(--transition);
  }
  .file-empty:hover { border-color: var(--accent); }
  .file-empty-icon { font-size: 36px; display: block; margin-bottom: 8px; }
  .file-empty p { color: var(--text-secondary); }
  .file-empty-hint { font-size: 12px; margin-top: 4px; color: var(--text-muted); }

  /* 文件列表 */
  .file-list { display: flex; flex-direction: column; gap: 6px; }
  .file-item {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px;
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    cursor: grab;
    transition: background var(--transition);
    user-select: none;
  }
  .file-item:hover { background: rgba(255,255,255,0.1); }
  .file-item.dragging { opacity: 0.5; }
  .drag-handle { color: var(--text-muted); font-size: 14px; }
  .file-index {
    width: 20px; height: 20px; border-radius: 50%;
    background: var(--accent); color: white;
    display: flex; align-items: center; justify-content: center;
    font-size: 11px; flex-shrink: 0;
  }
  .file-name {
    flex: 1; overflow: hidden; text-overflow: ellipsis;
    white-space: nowrap; font-size: 13px;
  }
  .btn-remove {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; font-size: 14px; padding: 2px;
  }
  .btn-remove:hover { color: #f87171; }
  .btn-add-more {
    background: none; border: 1px dashed var(--glass-border);
    color: var(--text-secondary); padding: 6px; border-radius: var(--radius-sm);
    cursor: pointer; font-size: 12px; margin-top: 4px;
  }
  .btn-add-more:hover { border-color: var(--accent); color: var(--accent); }

  /* 选项行 */
  .options-row {
    display: flex; gap: 16px; margin-top: 16px;
    flex-wrap: wrap;
  }
  .option-group {
    flex: 1; min-width: 200px;
  }
  .option-group label {
    display: block; font-size: 12px; color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .select-glass, .input-glass {
    width: 100%;
    background: rgba(255,255,255,0.08);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    padding: 8px 12px;
    font-size: 13px;
    outline: none;
  }
  .select-glass:focus, .input-glass:focus {
    border-color: var(--accent);
  }
  .select-glass option { background: #302b63; }
  .path-row { display: flex; gap: 8px; }
  .path-row .input-glass { flex: 1; }
  .btn-secondary {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
  }
  .btn-secondary:hover { background: var(--glass-hover); }

  .validation-msg {
    color: #fbbf24; font-size: 12px; margin-top: 8px;
  }

  /* --- 执行区 --- */
  .exec-placeholder {
    text-align: center; padding: 20px;
    color: var(--text-muted); font-size: 14px;
  }

  /* 进度 */
  .progress-section { margin-bottom: 16px; }
  .progress-bar {
    width: 100%; height: 8px;
    background: rgba(255,255,255,0.1);
    border-radius: 4px; overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), #a78bfa);
    border-radius: 4px;
    transition: width 0.3s ease;
  }
  .progress-info {
    display: flex; justify-content: space-between;
    font-size: 12px; color: var(--text-secondary);
    margin-top: 8px;
  }
  .btn-cancel {
    display: block; margin: 12px auto 0;
    background: rgba(248, 113, 113, 0.15);
    border: 1px solid rgba(248, 113, 113, 0.3);
    color: #f87171; padding: 8px 20px;
    border-radius: var(--radius-sm);
    cursor: pointer; font-size: 13px;
  }
  .btn-cancel:hover { background: rgba(248, 113, 113, 0.25); }

  /* 结果 */
  .result-success, .result-error {
    display: flex; align-items: flex-start; gap: 12px;
    margin-bottom: 16px;
  }
  .result-icon { font-size: 28px; }
  .result-title { font-weight: 600; margin-bottom: 4px; }
  .result-detail { font-size: 13px; color: var(--text-secondary); }
  .result-path {
    font-size: 12px; color: var(--text-muted);
    margin-top: 4px;
  }
  .result-actions {
    display: flex; gap: 8px; margin-top: 12px;
  }
  .btn-primary {
    background: var(--accent);
    border: none; color: white; padding: 8px 20px;
    border-radius: var(--radius-sm);
    cursor: pointer; font-size: 13px;
  }
  .btn-primary:hover { background: var(--accent-hover); }

  .btn-start {
    display: block; width: 100%;
    background: var(--accent); border: none;
    color: white; padding: 12px;
    border-radius: var(--radius-sm);
    font-size: 15px; cursor: pointer;
    transition: opacity var(--transition);
  }
  .btn-start:disabled {
    opacity: 0.4; cursor: not-allowed;
  }
  .btn-start:not(:disabled):hover {
    background: var(--accent-hover);
  }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add VideoConcat UI component with file drag-sort and progress"
```

---

### Task 4: 集成 — ToolView 路由 + 权限 + "打开文件夹" 命令

**文件:**
- Modify: `src/lib/pages/ToolView.svelte`
- Modify: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/commands/system.rs`（或追加到 video.rs）

**说明:** 将 VideoConcat 组件接入 ToolView，添加权限配置，实现 "打开所在文件夹" 功能。

- [ ] **Step 1: 更新 `ToolView.svelte` 渲染 VideoConcat 替换占位符**

```svelte
<script lang="ts">
  // ... existing imports ...
  import VideoConcat from '../components/VideoConcat.svelte';
  // ... existing code ...

  // 工具组件映射
  let toolComponent = $derived(found?.id);
</script>

<!-- ... existing header ... -->
<div class="tool-body">
  {#if found?.id === 'video-concat'}
    <VideoConcat />
  {:else}
    <p class="tool-placeholder">工具功能开发中…</p>
  {/if}
</div>
```

- [ ] **Step 2: 在 `commands/video.rs` 或新建 `commands/system.rs` 中添加 `show_item_in_folder` 命令**

```rust
#[tauri::command]
pub fn show_item_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }
    Ok(())
}
```

- [ ] **Step 3: 注册 `show_item_in_folder` 命令到 `lib.rs`**

```rust
.invoke_handler(tauri::generate_handler![
    commands::tools::get_tools,
    commands::video::check_ffmpeg,
    commands::video::concat_videos,
    commands::video::cancel_concat,
    commands::video::show_item_in_folder,
])
```

- [ ] **Step 4: 更新 `capabilities/default.json`，添加 dialog 和 event 权限**

```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "core:event:default",
    "core:event:allow-emit",
    "core:event:allow-listen"
  ]
}
```

（注：Tauri v2 中 dialog 权限可能需要具体化，实际根据构建报错调整）

- [ ] **Step 5: 编译 + 前端构建验证**

```bash
cd /Users/ripple/projects/trove2 && cargo build -p trove2 && npm run build
```

- [ ] **Step 6: 最终 Commit**

```bash
git add -A && git commit -m "feat: integrate VideoConcat with ToolView routing and permissions"
```

---

### 验证清单

- [ ] 启动应用，主页显示"视频拼接 🎬"工具卡片，分类为"转换"
- [ ] 点击进入工具页面，显示 ffmpeg 状态（已安装/未安装）
- [ ] 点击"选择视频文件"，系统文件对话框弹出
- [ ] 选择多个视频后，文件列表展示，支持拖拽排序
- [ ] 可移除单个文件或清空列表
- [ ] 输出格式可切换（MP4/MKV/MOV/AVI/WebM）
- [ ] 输出路径可自定义，默认为空（自动生成）
- [ ] 点击"开始拼接"，进度条实时更新（百分比/速度/ETA）
- [ ] 可取消正在进行的拼接
- [ ] 拼接成功后显示文件大小和耗时，点击"打开所在文件夹"定位文件
- [ ] 拼接失败显示具体错误信息
- [ ] 选择少于 2 个文件时"开始拼接"按钮禁用
- [ ] ffmpeg 未安装时全界面显示错误提示，"开始拼接"按钮禁用
