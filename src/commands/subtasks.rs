use chrono::Utc;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

pub fn add_subtask(state: &AppState, task_id: &str, title: &str) -> Result<Subtask> {
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

pub fn toggle_subtask(state: &AppState, task_id: &str, subtask_id: &str) -> Result<Subtask> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        s.done = !s.done;
        let s = s.clone();
        t.updated_at = Utc::now().to_rfc3339();
        s
    };
    state.save()?;
    Ok(updated)
}

pub fn delete_subtask(state: &AppState, task_id: &str, subtask_id: &str) -> Result<()> {
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

pub fn rename_subtask(state: &AppState, task_id: &str, subtask_id: &str, title: &str) -> Result<Subtask> {
    if title.trim().is_empty() {
        return Err(Error::InvalidInput("subtask title is empty".into()));
    }
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        let s = t.subtasks.iter_mut()
            .find(|s| s.id == subtask_id)
            .ok_or_else(|| Error::NotFound(format!("subtask {subtask_id}")))?;

        s.title = title.trim().to_string();
        let s = s.clone();
        t.updated_at = Utc::now().to_rfc3339();
        s
    };
    state.save()?;
    Ok(updated)
}

/// ドラッグ&ドロップ用。`from` 番目のサブタスクを `to` 番目の位置へ移す。
pub fn move_subtask_to(state: &AppState, task_id: &str, from: usize, to: usize) -> Result<Vec<Subtask>> {
    let subtasks = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;

        if from >= t.subtasks.len() {
            return Err(Error::InvalidInput(format!("subtask index {from}")));
        }
        let s = t.subtasks.remove(from);
        let to = to.min(t.subtasks.len());
        t.subtasks.insert(to, s);
        t.updated_at = Utc::now().to_rfc3339();
        t.subtasks.clone()
    };
    state.save()?;
    Ok(subtasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn move_subtask_to_reorders() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        let t = crate::commands::tasks::create_task(&s, NewTask { title: "a".into(), ..Default::default() }).unwrap();
        for n in ["1", "2", "3"] {
            add_subtask(&s, &t.id, n).unwrap();
        }
        let titles = |v: Vec<Subtask>| v.into_iter().map(|x| x.title).collect::<Vec<_>>();
        assert_eq!(titles(move_subtask_to(&s, &t.id, 0, 2).unwrap()), ["2", "3", "1"]);
        assert_eq!(titles(move_subtask_to(&s, &t.id, 2, 0).unwrap()), ["1", "2", "3"]);
    }
}
