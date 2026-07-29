use crate::models::tool::Tool;

#[tauri::command]
pub fn get_tools() -> Vec<Tool> {
    vec![]
}
