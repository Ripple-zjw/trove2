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
