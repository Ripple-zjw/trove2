use serde::Serialize;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tauri::Manager;

use crate::AppState;

#[derive(Debug, Serialize, Clone)]
pub struct FfmpegInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
pub struct ConcatProgress {
    pub percent: f64,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
pub struct ConcatResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VideoInfo {
    pub path: String,
    pub duration_us: u64,
    pub file_size: u64,
}

// ============ 辅助函数 ============

/// 确保输出文件路径具有正确的容器扩展名
fn resolve_output_path(output_path: &str, format: &str) -> String {
    let ext = match format {
        "mkv" => "mkv",
        "mov" => "mov",
        "avi" => "avi",
        "webm" => "webm",
        _ => "mp4",
    };
    let mut path = std::path::PathBuf::from(output_path);
    match path.extension() {
        Some(e) if e == ext => {}
        _ => {
            let _ = path.set_extension(ext);
        }
    }
    path.to_string_lossy().to_string()
}

/// 使用 ffprobe 获取所有输入文件的总时长（微秒）
fn get_total_duration_us(inputs: &[String]) -> Result<u64, String> {
    let mut total_us = 0u64;
    for input in inputs {
        let output = Command::new("ffprobe")
            .args(&[
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                input,
            ])
            .output()
            .map_err(|e| format!("无法执行 ffprobe: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffprobe 分析文件失败 '{}': {}", input, stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let duration_str = stdout.trim();
        if duration_str.is_empty() {
            return Err(format!("无法获取文件时长 '{}'", input));
        }

        let duration_secs: f64 = duration_str
            .parse()
            .map_err(|_| format!("无法解析时长 '{}' 为数字: {}", input, duration_str))?;

        total_us += (duration_secs * 1_000_000.0) as u64;
    }
    Ok(total_us)
}

/// 使用 ffprobe 检查所有输入文件的视频编码是否相同
fn check_all_same_codec(inputs: &[String]) -> Result<bool, String> {
    let mut first_codec: Option<String> = None;

    for input in inputs {
        let output = Command::new("ffprobe")
            .args(&[
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                input,
            ])
            .output()
            .map_err(|e| format!("无法执行 ffprobe: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffprobe 分析文件失败 '{}': {}", input, stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let codec = stdout.trim();

        if codec.is_empty() {
            return Err(format!("文件 '{}' 没有视频流", input));
        }

        match &first_codec {
            None => first_codec = Some(codec.to_string()),
            Some(c) if c == codec => {}
            Some(_) => return Ok(false),
        }
    }

    Ok(true)
}

/// 检查输入文件是否包含音频流。
fn has_audio_stream(input: &str) -> Result<bool, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input,
        ])
        .output()
        .map_err(|e| format!("无法执行 ffprobe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe 分析文件失败 '{}': {}", input, stderr));
    }

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// 判断 ffmpeg 错误信息是否属于需要回退到 reencode 的格式错误
fn is_format_error(stderr: &str) -> bool {
    let patterns = ["Non-monotonous DTS", "Invalid", "Packet mismatch"];
    patterns.iter().any(|p| stderr.contains(p))
}

// ============ 核心 ffmpeg 运行函数 ============

/// 共享的 ffmpeg 执行函数：启动 ffmpeg、解析 stdout 进度、发射事件、检查取消、等待完成、读取 stderr
fn run_ffmpeg(
    app_handle: &tauri::AppHandle,
    extra_args: &[String],
    cancel_flag: Arc<AtomicBool>,
    total_duration_us: u64,
    output_path: &str,
) -> Result<(ExitStatus, String), String> {
    let mut child = Command::new("ffmpeg");
    child
        .arg("-progress")
        .arg("pipe:1")
        .arg("-nostats")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for arg in extra_args {
        child.arg(arg);
    }

    let mut child = child
        .spawn()
        .map_err(|e| format!("无法启动 ffmpeg: {}", e))?;

    let stderr = child.stderr.take().ok_or("无法获取 ffmpeg stderr")?;
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut buf);
        buf
    });

    let stdout = child.stdout.take().ok_or("无法获取 ffmpeg stdout")?;
    let reader = BufReader::new(stdout);
    let start_time = Instant::now();

    // 逐行读取 stdout 中的进度信息
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        // 检查取消标志
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_reader.join();
            let _ = std::fs::remove_file(output_path);
            return Err("用户取消了拼接操作".to_string());
        }

        // 解析 out_time_us 字段计算进度
        if let Some(us_str) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = us_str.trim().parse::<u64>() {
                let elapsed = start_time.elapsed().as_secs_f64();
                let percent = if total_duration_us > 0 {
                    ((us as f64 / total_duration_us as f64) * 100.0).min(100.0)
                } else {
                    0.0
                };
                let speed = if elapsed > 0.0 {
                    format!("{:.2}x", us as f64 / 1_000_000.0 / elapsed)
                } else {
                    "0.00x".to_string()
                };
                let eta = if elapsed > 0.0 && percent > 0.0 {
                    let remaining_us = total_duration_us.saturating_sub(us) as f64;
                    let rate = us as f64 / elapsed; // 微秒/秒
                    let remaining_secs = if rate > 0.0 { remaining_us / rate } else { 0.0 };
                    format!("{:.0}s", remaining_secs)
                } else {
                    "N/A".to_string()
                };

                let _ = app_handle.emit(
                    "concat-progress",
                    ConcatProgress {
                        percent,
                        speed,
                        eta,
                    },
                );
            }
        }

        // 进度结束标记
        if line.trim() == "progress=end" {
            break;
        }
    }

    // 等待 ffmpeg 进程结束
    let status = child
        .wait()
        .map_err(|e| format!("等待 ffmpeg 完成失败: {}", e))?;

    // stderr 在进程运行期间已并发读取，避免其管道缓冲区写满而阻塞 ffmpeg。
    let stderr_output = stderr_reader.join().unwrap_or_default();

    Ok((status, stderr_output))
}

/// 使用 concat demuxer + copy 模式执行拼接
fn run_concat_copy(
    app_handle: &tauri::AppHandle,
    temp_list_path: &std::path::Path,
    output_path: &str,
    cancel_flag: Arc<AtomicBool>,
    total_duration_us: u64,
) -> Result<(ExitStatus, String), String> {
    let list_str = temp_list_path.to_str().ok_or("临时文件路径无效")?;

    let args = vec![
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_str.to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-y".to_string(),
        output_path.to_string(),
    ];

    run_ffmpeg(
        app_handle,
        &args,
        cancel_flag,
        total_duration_us,
        output_path,
    )
}

/// 使用 filter_complex concat 重新编码方式执行拼接
fn run_concat_reencode(
    app_handle: &tauri::AppHandle,
    inputs: &[String],
    output_path: &str,
    cancel_flag: Arc<AtomicBool>,
    total_duration_us: u64,
) -> Result<(ExitStatus, String), String> {
    let n = inputs.len();
    if n < 2 {
        return Err("至少需要两个输入文件才能拼接".to_string());
    }

    let mut args = Vec::new();

    // 添加输入文件
    for input in inputs {
        args.push("-i".to_string());
        args.push(input.clone());
    }

    let audio_streams = inputs
        .iter()
        .map(|input| has_audio_stream(input))
        .collect::<Result<Vec<_>, _>>()?;
    let has_audio = audio_streams[0];
    if audio_streams.iter().any(|&present| present != has_audio) {
        return Err("输入视频的音频轨道不一致；请使用全部带音频或全部无音频的视频".to_string());
    }

    // 无音轨视频使用 a=0，避免在 filter 中引用不存在的 [i:a] 流。
    let mut filter_parts = Vec::new();
    for i in 0..n {
        filter_parts.push(if has_audio {
            format!("[{}:v][{}:a]", i, i)
        } else {
            format!("[{}:v]", i)
        });
    }
    let filter_complex = if has_audio {
        format!(
            "{}concat=n={}:v=1:a=1[outv][outa]",
            filter_parts.join(""),
            n
        )
    } else {
        format!("{}concat=n={}:v=1:a=0[outv]", filter_parts.join(""), n)
    };

    args.push("-filter_complex".to_string());
    args.push(filter_complex);
    args.push("-map".to_string());
    args.push("[outv]".to_string());
    if has_audio {
        args.push("-map".to_string());
        args.push("[outa]".to_string());
    }
    args.push("-y".to_string());
    args.push(output_path.to_string());

    run_ffmpeg(
        app_handle,
        &args,
        cancel_flag,
        total_duration_us,
        output_path,
    )
}

// ============ 构建 ConcatResult 辅助 ============

fn build_concat_result_success(start: Instant, output: &str) -> ConcatResult {
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(output).ok().map(|m| m.len());
    ConcatResult {
        success: true,
        output_path: Some(output.to_string()),
        error: None,
        duration_ms: elapsed_ms,
        file_size,
    }
}

fn build_concat_result_failure(error: Option<String>, start: Instant) -> ConcatResult {
    let elapsed_ms = start.elapsed().as_millis() as u64;
    ConcatResult {
        success: false,
        output_path: None,
        error,
        duration_ms: elapsed_ms,
        file_size: None,
    }
}

// ============ Tauri 命令 ============

#[tauri::command]
pub fn concat_videos(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output_path: String,
    format: String,
) -> Result<ConcatResult, String> {
    let start = Instant::now();

    if inputs.len() < 2 {
        return Err("至少需要两个输入文件才能拼接".to_string());
    }

    // 0. 检查输入文件是否存在
    for input in &inputs {
        if !std::path::Path::new(input).exists() {
            return Err(format!("文件不存在: {}", input));
        }
    }

    // 1. 重置取消标志
    app.state::<AppState>()
        .cancel_flag
        .store(false, Ordering::SeqCst);
    let cancel_flag = app.state::<AppState>().cancel_flag.clone();

    // 2. 获取总时长
    let total_duration_us = get_total_duration_us(&inputs)?;

    // 3. 检查编码是否相同
    let same_codec = check_all_same_codec(&inputs)?;

    // 4. 解析输出路径（确保正确的容器扩展名）
    let output = resolve_output_path(&output_path, &format);

    // 5. 如果编码相同，尝试 concat demuxer + copy 模式
    if same_codec {
        // 创建临时列表文件
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let list_path = std::env::temp_dir().join(format!("trove2_concat_{}.txt", now));

        // 写入文件列表
        {
            let mut file = std::fs::File::create(&list_path)
                .map_err(|e| format!("无法创建临时文件: {}", e))?;
            for input in &inputs {
                // concat demuxer 要求路径用单引号包裹，路径内的单引号使用转义序列
                let escaped = input.replace("'", "'\\''");
                writeln!(file, "file '{}'", escaped)
                    .map_err(|e| format!("无法写入临时文件: {}", e))?;
            }
        }

        // 尝试 copy 模式
        let copy_result = run_concat_copy(
            &app,
            &list_path,
            &output,
            cancel_flag.clone(),
            total_duration_us,
        );

        // 立即清理临时列表文件
        let _ = std::fs::remove_file(&list_path);

        match copy_result {
            Ok((status, _)) if status.success() => {
                // copy 成功
                return Ok(build_concat_result_success(start, &output));
            }
            Ok((_, stderr)) => {
                // copy 运行但失败
                if is_format_error(&stderr) {
                    // 格式错误 → 回退到 reencode
                } else if cancel_flag.load(Ordering::SeqCst) {
                    // 用户取消了，直接返回失败结果
                    let _ = std::fs::remove_file(&output);
                    return Ok(build_concat_result_failure(
                        Some("用户取消了拼接操作".to_string()),
                        start,
                    ));
                }
                // 其他错误 → 尝试 reencode 回退
            }
            Err(e) => {
                // ffmpeg 无法启动或被取消
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = std::fs::remove_file(&output);
                    return Err(e);
                }
                // 其他启动失败也尝试 reencode
            }
        }
    }

    // 6. 使用 reencode 模式（编码不同或 copy 失败需要回退）
    match run_concat_reencode(
        &app,
        &inputs,
        &output,
        cancel_flag.clone(),
        total_duration_us,
    ) {
        Ok((status, _stderr)) if status.success() => {
            Ok(build_concat_result_success(start, &output))
        }
        Ok((_, stderr)) => Ok(build_concat_result_failure(Some(stderr), start)),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn cancel_concat(state: tauri::State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::SeqCst);
}

// ============ 获取视频文件信息 ============

#[tauri::command]
pub fn get_video_info(path: String) -> Result<VideoInfo, String> {
    // 获取文件大小
    let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取文件信息: {}", e))?;
    let file_size = metadata.len();

    // 通过 ffprobe 获取时长
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &path,
        ])
        .output()
        .map_err(|e| format!("无法执行 ffprobe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe 分析失败 '{}': {}", path, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration_secs: f64 = stdout
        .trim()
        .parse()
        .map_err(|_| format!("无法解析视频时长: {}", path))?;
    let duration_us = (duration_secs * 1_000_000.0) as u64;

    Ok(VideoInfo {
        path,
        duration_us,
        file_size,
    })
}

// ============ 原有代码 ============

#[tauri::command]
pub fn check_ffmpeg() -> FfmpegInfo {
    // 查找 ffmpeg 路径
    let path = which_ffmpeg();

    if let Some(path_str) = &path {
        let output = Command::new("ffmpeg").arg("-version").output();

        match output {
            Ok(out) if out.status.success() => {
                let version_str = String::from_utf8_lossy(&out.stdout);
                let version_str_err = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", version_str, version_str_err);
                let version_line = combined.lines().next().unwrap_or("").to_string();
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
    let which_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    Command::new(which_cmd)
        .arg("ffmpeg")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

// ============ 系统命令 ============

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
        let parent = std::path::Path::new(&path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }
    Ok(())
}
