mod commands;
mod models;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::tools::get_tools,
            commands::video::check_ffmpeg,
            commands::video::concat_videos,
            commands::video::cancel_concat,
            commands::video::show_item_in_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
