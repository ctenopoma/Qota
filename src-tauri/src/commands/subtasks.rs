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
        estimated_minutes: None,
        actual_minutes: None,
        started_at: None,
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

#[tauri::command]
pub fn update_subtask(
    state: State<'_, AppState>,
    task_id: String,
    subtask_id: String,
    patch: crate::models::SubtaskPatch,
) -> Result<Subtask> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        if let Some(title) = patch.title {
            if title.trim().is_empty() {
                return Err(Error::InvalidInput("subtask title is empty".into()));
            }
            s.title = title.trim().to_string();
        }
        if let Some(est) = patch.estimated_minutes {
            s.estimated_minutes = est;
        }
        if let Some(act) = patch.actual_minutes {
            s.actual_minutes = act;
        }
        if let Some(started) = patch.started_at {
            s.started_at = started;
        }
        t.updated_at = Utc::now().to_rfc3339();
        s.clone()
    };

    // Auto-calculate task times from subtasks (after releasing mutable borrow)
    {
        let mut data = state.data.lock();
        if let Some(t) = data.tasks.iter_mut().find(|t| t.id == task_id) {
            let has_subtask_estimates = t.subtasks.iter().any(|st| st.estimated_minutes.is_some());
            if has_subtask_estimates {
                let total_est: u32 = t.subtasks.iter()
                    .filter_map(|st| st.estimated_minutes)
                    .sum();
                t.estimated_minutes = if total_est > 0 { Some(total_est) } else { None };
            }

            let has_subtask_actuals = t.subtasks.iter().any(|st| st.actual_minutes.is_some());
            if has_subtask_actuals {
                let total_act: u32 = t.subtasks.iter()
                    .filter_map(|st| st.actual_minutes)
                    .sum();
                t.actual_minutes = if total_act > 0 { Some(total_act) } else { None };
            }
        }
    }

    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn move_subtask(
    state: State<'_, AppState>,
    task_id: String,
    subtask_id: String,
    direction: String,
) -> Result<Vec<Subtask>> {
    let subtasks = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let idx = t.subtasks.iter().position(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        match direction.as_str() {
            "up" if idx > 0 => t.subtasks.swap(idx, idx - 1),
            "down" if idx + 1 < t.subtasks.len() => t.subtasks.swap(idx, idx + 1),
            _ => {}
        }
        t.updated_at = Utc::now().to_rfc3339();
        t.subtasks.clone()
    };
    state.save()?;
    Ok(subtasks)
}

#[tauri::command]
pub fn start_subtask_timer(state: State<'_, AppState>, task_id: String, subtask_id: String) -> Result<Subtask> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        s.started_at = Some(Utc::now().to_rfc3339());
        t.updated_at = Utc::now().to_rfc3339();
        s.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn stop_subtask_timer(state: State<'_, AppState>, task_id: String, subtask_id: String) -> Result<Subtask> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        if let Some(started) = s.started_at.take() {
            let elapsed = (Utc::now()
                .signed_duration_since(chrono::DateTime::parse_from_rfc3339(&started)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()))
                .num_seconds() as u32 + 59) / 60;
            let new_actual = (s.actual_minutes.unwrap_or(0)) + elapsed.max(1);
            s.actual_minutes = Some(new_actual);
        }

        t.updated_at = Utc::now().to_rfc3339();
        s.clone()
    };

    // Auto-calculate task actual_minutes from subtasks (after releasing mutable borrow)
    {
        let mut data = state.data.lock();
        if let Some(t) = data.tasks.iter_mut().find(|t| t.id == task_id) {
            let has_subtask_actuals = t.subtasks.iter().any(|st| st.actual_minutes.is_some());
            if has_subtask_actuals {
                let total_act: u32 = t.subtasks.iter()
                    .filter_map(|st| st.actual_minutes)
                    .sum();
                t.actual_minutes = if total_act > 0 { Some(total_act) } else { None };
            }
        }
    }

    state.save()?;
    Ok(updated)
}
