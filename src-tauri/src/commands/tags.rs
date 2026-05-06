use chrono::Utc;
use tauri::State;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

#[tauri::command]
pub fn create_tag(state: State<'_, AppState>, name: String) -> Result<Tag> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::InvalidInput("tag name is empty".into()));
    }
    
    let now = Utc::now().to_rfc3339();
    let tag = Tag {
        id: format!("t_{}", Uuid::new_v4().simple()),
        name: name.clone(),
        created_at: now,
    };
    
    {
        let mut data = state.data.lock();
        if data.tags.iter().any(|t| t.name.eq_ignore_ascii_case(&name)) {
            return Err(Error::InvalidInput("tag already exists".into()));
        }
        data.tags.push(tag.clone());
    }
    state.save()?;
    Ok(tag)
}

#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, id: String) -> Result<()> {
    {
        let mut data = state.data.lock();
        let len = data.tags.len();
        data.tags.retain(|t| t.id != id);
        if data.tags.len() == len {
            return Err(Error::NotFound(format!("tag {id}")));
        }
        
        // Remove tag_id from associated tasks
        let now = Utc::now().to_rfc3339();
        for task in data.tasks.iter_mut() {
            if task.tag_ids.contains(&id) {
                task.tag_ids.retain(|tid| tid != &id);
                task.updated_at = now.clone();
            }
        }
    }
    state.save()
}
