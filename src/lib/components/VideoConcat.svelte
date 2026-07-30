<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, save, ask } from '@tauri-apps/plugin-dialog';
  import GlassPanel from './GlassPanel.svelte';

  // --- 类型 ---
  interface FileInfo {
    uid: string;
    path: string;
    name: string;
    duration_us: number;
    file_size: number;
    loading: boolean;
    error?: string;
  }

  type SortField = 'order' | 'duration' | 'size';

  // --- 状态 ---
  let ffmpegInfo = $state<{ installed: boolean; version?: string; path?: string } | null>(null);
  let ffmpegLoading = $state(true);

  let files = $state<FileInfo[]>([]);
  let outputFormat = $state('mp4');
  let outputPath = $state('');
  let dropTargetIndex = $state<number | null>(null);
  // WKWebView 不支持 HTML5 DragEvent，用鼠标事件模拟拖拽
  let dragState = $state<{
    source: number;
    startY: number;
    dragging: boolean;
  } | null>(null);
  let sortField = $state<SortField>('order');
  let sortAsc = $state(true);
  let _uidCounter = 0;
  function nextUid() { return `f${++_uidCounter}`; }

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
    if (!selected || selected.length === 0) return;

    // 检测重复：与已有文件列表中的 path 对比，或选中文件自身重复
    const existingPaths = new Set(files.map(f => f.path));
    const inList = selected.filter(p => existingPaths.has(p));
    const selfDup = selected.filter((p, i) => selected.indexOf(p) !== i);
    const hasDuplicates = inList.length > 0 || selfDup.length > 0;

    let pathsToAdd = selected;
    if (hasDuplicates) {
      const parts: string[] = [];
      if (inList.length > 0) parts.push(`${inList.length} 个已在列表中`);
      if (selfDup.length > 0) parts.push(`${selfDup.length} 个自身重复`);
      const answer = await ask(
        `检测到重复视频（${parts.join('，')}）。\n是否去掉重复的文件？`,
        { title: '重复视频检测', okLabel: '去掉重复', cancelLabel: '全部保留' }
      );
      if (answer) {
        // 去掉重复：移除已在列表中的和自身重复的
        const seen = new Set(files.map(f => f.path));
        pathsToAdd = selected.filter(p => {
          if (seen.has(p)) return false;
          seen.add(p);
          return true;
        });
      }
      // answer === false 时保留所有（包括重复），pathsToAdd 保持原样
    }

    if (pathsToAdd.length === 0) return;

    const newInfos: FileInfo[] = pathsToAdd.map(p => ({
      uid: nextUid(),
      path: p,
      name: getFileName(p),
      duration_us: 0,
      file_size: 0,
      loading: true,
    }));
    files = [...files, ...newInfos];
    fetchVideoInfos(pathsToAdd);
  }

  async function fetchVideoInfos(paths: string[]) {
    for (const path of paths) {
      try {
        const info = await invoke<{ path: string; duration_us: number; file_size: number }>('get_video_info', { path });
        files = files.map(f =>
          f.path === path
            ? { ...f, duration_us: info.duration_us, file_size: info.file_size, loading: false }
            : f
        );
      } catch (e) {
        files = files.map(f =>
          f.path === path
            ? { ...f, loading: false, error: String(e) }
            : f
        );
      }
    }
  }

  function removeFile(index: number) {
    files = files.filter((_, i) => i !== index);
  }

  function clearFiles() {
    files = [];
    result = null;
    sortField = 'order';
    sortAsc = true;
  }

  function defaultOutputPath() {
    const firstPath = files[0]?.path;
    const separatorIndex = firstPath
      ? Math.max(firstPath.lastIndexOf('/'), firstPath.lastIndexOf('\\'))
      : -1;
    const directory = separatorIndex >= 0 ? firstPath!.slice(0, separatorIndex + 1) : '';
    return `${directory}concat_${Date.now()}.${outputFormat}`;
  }

  // --- 拖拽排序 (鼠标事件模拟，兼容 WKWebView) ---
  // WKWebView (Tauri macOS) 对 HTML5 Drag & Drop API 支持不全，
  // dragenter/dragover/drop 事件不可靠，改用 mousedown/mousemove/mouseup 实现。
  function sortable(node: HTMLElement, index: number) {
    let currentIndex = index;

    const onMouseDown = (e: MouseEvent) => {
      // 忽略移除按钮上的点击
      if ((e.target as HTMLElement).closest('button')) return;
      e.preventDefault();
      dragState = { source: currentIndex, startY: e.clientY, dragging: true };
      dropTargetIndex = currentIndex;
    };

    node.addEventListener('mousedown', onMouseDown);

    return {
      update(newIndex: number) {
        currentIndex = newIndex;
      },
      destroy() {
        node.removeEventListener('mousedown', onMouseDown);
      },
    };
  }

  // 全局 mouse 事件：在 dragState 激活时跟踪鼠标位置计算 drop target
  $effect(() => {
    if (!dragState?.dragging) return;

    const onMouseMove = (e: MouseEvent) => {
      // 遍历所有 .file-item 元素，找到鼠标 Y 坐标覆盖的那个
      const items = document.querySelectorAll('.file-list .file-item');
      let found = false;
      for (let i = 0; i < items.length; i++) {
        const rect = items[i].getBoundingClientRect();
        if (e.clientY >= rect.top && e.clientY <= rect.bottom) {
          dropTargetIndex = i;
          found = true;
          break;
        }
      }
      if (!found) dropTargetIndex = null;
    };

    const onMouseUp = () => {
      const src = dragState?.source;
      const dst = dropTargetIndex;
      if (src != null && dst != null && src !== dst) {
        const newFiles = [...files];
        const [moved] = newFiles.splice(src, 1);
        newFiles.splice(dst, 0, moved);
        files = newFiles;
      }
      dragState = null;
      dropTargetIndex = null;
    };

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    return () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
  });

  // --- 排序 (点击后直接重排 files 数组) ---
  function toggleSort(field: SortField) {
    if (field === 'order') {
      sortField = 'order';
      return;
    }
    const asc = field === sortField ? !sortAsc : true;
    sortField = field;
    sortAsc = asc;
    files = [...files].sort((a, b) => {
      let cmp: number;
      if (field === 'duration') {
        cmp = a.duration_us - b.duration_us;
      } else {
        cmp = Number(a.file_size) - Number(b.file_size);
      }
      return asc ? cmp : -cmp;
    });
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

  // --- 格式化 ---
  function formatFileSize(bytes?: number): string {
    if (!bytes) return '--';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function formatDuration(us?: number): string {
    if (!us) return '--';
    const totalSec = Math.floor(us / 1_000_000);
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function getFileName(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1] || path;
  }

  // --- 拼接 ---
  async function startConcat() {
    if (!canStart) return;
    result = null;
    running = true;
    progress = null;

    const unlisten = await listen<{ percent: number; speed: string; eta: string }>('concat-progress', (event) => {
      progress = event.payload;
    });

    try {
      const res = await invoke('concat_videos', {
        inputs: files.map(f => f.path),
        outputPath: outputPath || defaultOutputPath(),
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
    return !running
      && ffmpegInfo?.installed
      && files.length >= 2
      && files.every((file) => !file.loading && !file.error);
  });

  let validationMsg = $derived.by(() => {
    if (files.length < 2 && files.length > 0) return '至少选择 2 个视频文件';
    if (files.some((file) => file.loading)) return '正在读取视频信息，请稍候';
    if (files.some((file) => file.error)) return '请移除无法读取的视频文件后再开始';
    return '';
  });
</script>

<div class="video-concat">
  <!-- ① ffmpeg 状态 -->
  <GlassPanel padding="16px">
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
  <GlassPanel padding="20px">
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
      <div class="file-list-header">
        <span class="hdr-index">#</span>
        <span class="hdr-name">文件名</span>
        <button class="hdr-sort" onclick={() => toggleSort('duration')}>
          时长 {sortField === 'duration' ? (sortAsc ? '↑' : '↓') : ''}
        </button>
        <button class="hdr-sort" onclick={() => toggleSort('size')}>
          大小 {sortField === 'size' ? (sortAsc ? '↑' : '↓') : ''}
        </button>
        <span class="hdr-action"></span>
      </div>
      <div class="file-list" role="list">
        {#each files as file, i (file.uid)}
          <div
            class="file-item"
            class:dragging={dragState?.source === i && dragState?.dragging}
            class:drop-target={dropTargetIndex === i}
            role="listitem"
            use:sortable={i}
          >
            <span class="drag-handle" aria-hidden="true">⠿</span>
            <span class="file-index">{i + 1}</span>
            <span class="file-name" title={file.path}>{file.name}</span>
            <span class="file-duration">
              {#if file.loading}
                <span class="loading-spinner"></span>
              {:else if file.error}
                <span class="file-error" title={file.error}>⚠️</span>
              {:else}
                {formatDuration(file.duration_us)}
              {/if}
            </span>
            <span class="file-size">
              {#if file.loading}
                <span class="loading-spinner"></span>
              {:else if !file.error}
                {formatFileSize(file.file_size)}
              {/if}
            </span>
            <button class="btn-remove" onclick={() => removeFile(i)} title="移除">✕</button>
          </div>
        {/each}
        <button class="btn-add-more" onclick={selectFiles}>+ 添加更多文件</button>
      </div>
    {/if}

    <div class="options-row">
      <div class="option-group">
        <label for="output-format">输出格式</label>
        <select id="output-format" bind:value={outputFormat} class="select-glass">
          {#each formats as fmt}
            <option value={fmt.value}>{fmt.label}</option>
          {/each}
        </select>
      </div>
      <div class="option-group">
        <label for="output-path">输出路径</label>
        <div class="path-row">
          <input
            id="output-path"
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
  <GlassPanel padding="20px">
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
              大小: {formatFileSize(result.file_size)} | 耗时: {result.duration_ms ? `${(result.duration_ms / 1000).toFixed(1)} 秒` : '--'}
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
  .status-loading { color: var(--text-muted); font-size: 14px; }
  .status-ok, .status-err { display: flex; align-items: flex-start; gap: 12px; font-size: 14px; }
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
    outline: none;
  }
  .file-empty:hover, .file-empty:focus-visible { border-color: var(--accent); }
  .file-empty-icon { font-size: 36px; display: block; margin-bottom: 8px; }
  .file-empty p { color: var(--text-secondary); }
  .file-empty-hint { font-size: 12px; margin-top: 4px; color: var(--text-muted); }

  /* 文件列表表头 */
  .file-list-header {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 12px 6px 40px;
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .hdr-index { width: 20px; flex-shrink: 0; text-align: center; }
  .hdr-name { flex: 1; }
  .hdr-sort {
    width: 80px; flex-shrink: 0; text-align: right;
    background: none; border: none;
    color: var(--text-muted); cursor: pointer;
    font-size: 11px; text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: color var(--transition);
  }
  .hdr-sort:hover { color: var(--accent); }
  .hdr-action { width: 24px; flex-shrink: 0; }

  /* 文件列表 */
  .file-list { display: flex; flex-direction: column; gap: 4px; }
  .file-item {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px;
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    cursor: grab;
    user-select: none;
    transition: background var(--transition), border-color var(--transition), opacity var(--transition);
  }
  .file-item:hover { background: rgba(255,255,255,0.1); }
  .file-item.dragging { opacity: 0.4; }
  .file-item.drop-target {
    border-color: var(--accent);
    background: rgba(124, 108, 240, 0.1);
  }
  .drag-handle { color: var(--text-muted); font-size: 14px; cursor: grab; flex-shrink: 0; }
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
  .file-duration { width: 80px; flex-shrink: 0; text-align: right; font-size: 12px; color: var(--text-secondary); }
  .file-size { width: 80px; flex-shrink: 0; text-align: right; font-size: 12px; color: var(--text-secondary); }
  .file-error { cursor: help; }
  .btn-remove {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; font-size: 14px; padding: 2px; width: 24px; flex-shrink: 0;
  }
  .btn-remove:hover { color: #f87171; }

  .loading-spinner {
    display: inline-block;
    width: 12px; height: 12px;
    border: 2px solid var(--text-muted);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

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
