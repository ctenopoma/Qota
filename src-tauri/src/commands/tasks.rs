use chrono::Utc;
use tauri::State;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

#[tauri::command]
pub fn create_task(state: State<'_, AppState>, input: NewTask) -> Result<Task> {
    if input.title.trim().is_empty() {
        return Err(Error::InvalidInput("title is empty".into()));
    }
    let now = Utc::now().to_rfc3339();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title: input.title.trim().to_string(),
        notes: input.notes.unwrap_or_default(),
        status: TaskStatus::Todo,
        priority: input.priority,
        due: input.due,
        project_id: input.project_id,
        tag_ids: input.tag_ids.unwrap_or_default(),
        subtasks: vec![],
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };
    {
        let mut data = state.data.lock();
        data.tasks.push(task.clone());
    }
    state.save()?;
    Ok(task)
}

#[tauri::command]
pub fn update_task(state: State<'_, AppState>, id: String, patch: TaskPatch) -> Result<Task> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(format!("task {id}")))?;

        if let Some(v) = patch.title    { t.title = v; }
        if let Some(v) = patch.notes    { t.notes = v; }
        if let Some(v) = patch.status   { t.status = v; }
        if let Some(v) = patch.priority { t.priority = v; }
        if let Some(v) = patch.due      { t.due = v; }
        if let Some(v) = patch.project_id { t.project_id = v; }
        if let Some(v) = patch.tag_ids  { t.tag_ids = v; }
        t.updated_at = Utc::now().to_rfc3339();
        t.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn toggle_task(state: State<'_, AppState>, id: String) -> Result<Task> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(format!("task {id}")))?;
        let now = Utc::now().to_rfc3339();
        match t.status {
            TaskStatus::Todo => { t.status = TaskStatus::Done; t.completed_at = Some(now.clone()); }
            TaskStatus::Done => { t.status = TaskStatus::Todo; t.completed_at = None; }
        }
        t.updated_at = now;
        t.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_task(state: State<'_, AppState>, id: String) -> Result<()> {
    {
        let mut data = state.data.lock();
        let len = data.tasks.len();
        data.tasks.retain(|t| t.id != id);
        if data.tasks.len() == len {
            return Err(Error::NotFound(format!("task {id}")));
        }
    }
    state.save()
}
