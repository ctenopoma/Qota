use chrono::Utc;
use tauri::State;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

#[tauri::command]
pub fn create_project(state: State<'_, AppState>, input: NewProject) -> Result<Project> {
    if input.name.trim().is_empty() {
        return Err(Error::InvalidInput("project name is empty".into()));
    }
    let now = Utc::now().to_rfc3339();
    let color = input.color.unwrap_or_else(|| "oklch(0.7 0.1 200)".into()); // default color if none
    
    let project = Project {
        id: format!("p_{}", Uuid::new_v4().simple()),
        name: input.name.trim().to_string(),
        color,
        archived: false,
        created_at: now,
    };
    
    {
        let mut data = state.data.lock();
        data.projects.push(project.clone());
    }
    state.save()?;
    Ok(project)
}

#[tauri::command]
pub fn update_project(state: State<'_, AppState>, id: String, patch: ProjectPatch) -> Result<Project> {
    let updated = {
        let mut data = state.data.lock();
        let p = data.projects.iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| Error::NotFound(format!("project {id}")))?;

        if let Some(v) = patch.name { p.name = v; }
        if let Some(v) = patch.color { p.color = v; }
        if let Some(v) = patch.archived { p.archived = v; }
        
        p.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: String) -> Result<()> {
    {
        let mut data = state.data.lock();
        let len = data.projects.len();
        data.projects.retain(|p| p.id != id);
        if data.projects.len() == len {
            return Err(Error::NotFound(format!("project {id}")));
        }
        
        // Remove project_id from associated tasks
        let now = Utc::now().to_rfc3339();
        for task in data.tasks.iter_mut() {
            if task.project_id.as_deref() == Some(&id) {
                task.project_id = None;
                task.updated_at = now.clone();
            }
        }
    }
    state.save()
}
