use serde::Serialize;
use std::process::Command;

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
