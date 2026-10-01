use chrono::Utc;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

pub fn create_tag(state: &AppState, name: &str) -> Result<Tag> {
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

pub fn delete_tag(state: &AppState, id: &str) -> Result<()> {
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
            if task.tag_ids.iter().any(|tid| tid == id) {
                task.tag_ids.retain(|tid| tid != id);
                task.updated_at = now.clone();
            }
        }
    }
    state.save()
}
