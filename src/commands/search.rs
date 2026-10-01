use crate::models::Task;
use crate::store::AppState;

pub fn search(state: &AppState, q: &str) -> Vec<Task> {
    let needle = q.to_lowercase();
    if needle.trim().is_empty() {
        return vec![];
    }
    let data = state.data.lock();
    data.tasks.iter()
        .filter(|t|
            t.title.to_lowercase().contains(&needle)
            || t.notes.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}
