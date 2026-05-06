use chrono::Utc;
use tauri::State;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

#[tauri::command]
pub fn add_subtask(state: State<'_, AppState>, task_id: String, title: String) -> Result<Subtask> {
    if title.trim().is_empty() {
        return Err(Error::InvalidInput("subtask title is empty".into()));
    }
    let subtask = Subtask {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        done: false,
    };
    {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
        t.subtasks.push(subtask.clone());
        t.updated_at = Utc::now().to_rfc3339();
    }
    state.save()?;
    Ok(subtask)
}

#[tauri::command]
pub fn toggle_subtask(state: State<'_, AppState>, task_id: String, subtask_id: String) -> Result<Subtask> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
        
        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;
            
        s.done = !s.done;
        t.updated_at = Utc::now().to_rfc3339();
        s.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_subtask(state: State<'_, AppState>, task_id: String, subtask_id: String) -> Result<()> {
    {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
            
        let len = t.subtasks.len();
        t.subtasks.retain(|s| s.id != subtask_id);
        if t.subtasks.len() == len {
            return Err(Error::NotFound(format!("subtask {subtask_id}")));
        }
        t.updated_at = Utc::now().to_rfc3339();
    }
    state.save()
}
