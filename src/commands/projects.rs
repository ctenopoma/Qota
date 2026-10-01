use chrono::Utc;
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

pub fn create_project(state: &AppState, input: NewProject) -> Result<Project> {
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

pub fn update_project(state: &AppState, id: &str, patch: ProjectPatch) -> Result<Project> {
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

/// サイドバーのドラッグ&ドロップ用。id のプロジェクトを before の直前へ移す（None なら末尾）。
/// サイドバーはアーカイブ済みを出さないので、位置ではなく id で指定する
pub fn move_project_before(state: &AppState, id: &str, before: Option<&str>) -> Result<()> {
    if before == Some(id) {
        return Ok(());
    }
    {
        let mut data = state.data.lock();
        let from = data.projects.iter().position(|p| p.id == id)
            .ok_or_else(|| Error::NotFound(format!("project {id}")))?;
        let project = data.projects.remove(from);
        let to = match before {
            Some(b) => data.projects.iter().position(|p| p.id == b).unwrap_or(data.projects.len()),
            None => data.projects.len(),
        };
        data.projects.insert(to, project);
        // 自分の直前・直後への移動は並びが変わらないので保存しない（取り消しの履歴にも積まれない）
        if to == from {
            return Ok(());
        }
    }
    state.save()
}

pub fn delete_project(state: &AppState, id: &str) -> Result<()> {
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
            if task.project_id.as_deref() == Some(id) {
                task.project_id = None;
                task.updated_at = now.clone();
            }
        }
    }
    state.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn move_project_before_reorders_by_id() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        s.data.lock().projects.clear();
        let ids: Vec<String> = ["a", "b", "c"].iter()
            .map(|n| create_project(&s, NewProject { name: (*n).into(), color: None }).unwrap().id)
            .collect();
        let names = |s: &AppState| s.data.lock().projects.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        move_project_before(&s, &ids[2], Some(&ids[0])).unwrap();
        assert_eq!(names(&s), ["c", "a", "b"]);
        move_project_before(&s, &ids[2], None).unwrap();
        assert_eq!(names(&s), ["a", "b", "c"]);
        move_project_before(&s, &ids[0], Some(&ids[2])).unwrap();
        assert_eq!(names(&s), ["b", "a", "c"]);
        // 自分自身の前へは動かさない
        move_project_before(&s, &ids[0], Some(&ids[0])).unwrap();
        assert_eq!(names(&s), ["b", "a", "c"]);
    }
}
