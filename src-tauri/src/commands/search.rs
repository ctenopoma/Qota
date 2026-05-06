use tauri::State;
use crate::error::Result;
use crate::models::Task;
use crate::store::AppState;

#[tauri::command]
pub fn search(state: State<'_, AppState>, q: String) -> Result<Vec<Task>> {
    let needle = q.to_lowercase();
    if needle.trim().is_empty() {
        return Ok(vec![]);
    }
    let data = state.data.lock();
    let hits: Vec<Task> = data.tasks.iter()
        .filter(|t|
            t.title.to_lowercase().contains(&needle)
            || t.notes.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    Ok(hits)
}
