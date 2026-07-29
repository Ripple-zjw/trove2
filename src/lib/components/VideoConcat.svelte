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
