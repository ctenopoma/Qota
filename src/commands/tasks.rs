use chrono::{Datelike, Days, NaiveDate, Utc, Weekday};
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::*;
use crate::store::AppState;

fn calculate_next_date(date_str: &str, interval: &Interval) -> Option<String> {
    let mut parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    parsed = match interval {
        Interval::Daily => parsed + Days::new(1),
        Interval::Weekly => parsed + Days::new(7),
        Interval::Monthly => {
            let mut y = parsed.year();
            let mut m = parsed.month() + 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
            let d = parsed.day();
            NaiveDate::from_ymd_opt(y, m, d).unwrap_or_else(|| {
                let next_m = if m == 12 { 1 } else { m + 1 };
                let next_y = if m == 12 { y + 1 } else { y };
                NaiveDate::from_ymd_opt(next_y, next_m, 1).unwrap().pred_opt().unwrap()
            })
        }
        Interval::Yearly => {
            let y = parsed.year() + 1;
            let m = parsed.month();
            let d = parsed.day();
            NaiveDate::from_ymd_opt(y, m, d).unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(y, m, 28).unwrap()
            })
        }
        Interval::Weekdays => {
            let mut next = parsed + Days::new(1);
            while next.weekday() == Weekday::Sat || next.weekday() == Weekday::Sun {
                next = next + Days::new(1);
            }
            next
        }
    };
    Some(parsed.format("%Y-%m-%d").to_string())
}

pub fn create_task(state: &AppState, input: NewTask) -> Result<Task> {
    if input.title.trim().is_empty() {
        return Err(Error::InvalidInput("title is empty".into()));
    }
    let now = Utc::now().to_rfc3339();
    let mut task = Task {
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
        interval: input.interval,
        recurrence_id: input.recurrence_id.clone(),
        recurrence_end: input.recurrence_end.clone(),
    };

    // If it has recurrence set, ensure it has a recurrence_id
    if task.interval.is_some() && task.recurrence_id.is_none() {
        task.recurrence_id = Some(Uuid::new_v4().to_string());
    }

    let mut generated = vec![];
    {
        let mut data = state.data.lock();
        data.tasks.push(task.clone());

        // Generate future instances if interval and end_date are set
        if let (Some(interval), Some(due), Some(end_date), Some(_rid)) = (&task.interval, &task.due, &task.recurrence_end, &task.recurrence_id) {
            let mut current_due = due.clone();
            while let Some(next_due) = calculate_next_date(&current_due, interval) {
                if next_due > *end_date {
                    break;
                }
                let mut nt = task.clone();
                nt.id = Uuid::new_v4().to_string();
                nt.due = Some(next_due.clone());
                generated.push(nt);
                current_due = next_due;
            }
        }
        for nt in &generated {
            data.tasks.push(nt.clone());
        }
    }
    state.save()?;
    Ok(task)
}

pub fn update_task(state: &AppState, id: &str, patch: TaskPatch) -> Result<Vec<Task>> {
    let mut generated = vec![];
    let updated = {
        let mut data = state.data.lock();

        let t_idx = data.tasks.iter().position(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(format!("task {id}")))?;

        let old_recurrence_id = data.tasks[t_idx].recurrence_id.clone();

        let series_changed = patch.title.is_some()
            || patch.notes.is_some()
            || patch.priority.is_some()
            || patch.due.is_some()
            || patch.project_id.is_some()
            || patch.tag_ids.is_some()
            || patch.interval.is_some()
            || patch.recurrence_end.is_some();

        {
            let t = &mut data.tasks[t_idx];
            if let Some(v) = patch.title    { t.title = v; }
            if let Some(v) = patch.notes    { t.notes = v; }
            if let Some(v) = patch.status   { t.status = v; }
            if let Some(v) = patch.priority { t.priority = v; }
            if let Some(v) = patch.due      { t.due = v; }
            if let Some(v) = patch.project_id { t.project_id = v; }
            if let Some(v) = patch.tag_ids  { t.tag_ids = v; }
            if let Some(v) = patch.interval { t.interval = v; }
            if let Some(v) = patch.recurrence_end { t.recurrence_end = v; }
            t.updated_at = Utc::now().to_rfc3339();

            // Assign recurrence_id if turning on recurrence for the first time
            if t.interval.is_some() && t.recurrence_id.is_none() {
                t.recurrence_id = Some(Uuid::new_v4().to_string());
            }

            if t.interval.is_none() {
                t.recurrence_id = None;
                t.recurrence_end = None;
            }
        }

        let t = data.tasks[t_idx].clone();

        if series_changed {
            if let Some(rid) = &old_recurrence_id {
                let current_due = t.due.clone().unwrap_or_default();
                data.tasks.retain(|x| {
                    if x.id == t.id { return true; }
                    if x.recurrence_id.as_deref() == Some(rid) && x.status == TaskStatus::Todo {
                        let x_due = x.due.clone().unwrap_or_default();
                        if x_due > current_due {
                            return false;
                        }
                    }
                    true
                });
            }

            if let Some(_rid) = &t.recurrence_id {
                if let (Some(interval), Some(due), Some(end_date)) = (&t.interval, &t.due, &t.recurrence_end) {
                    let mut current_due = due.clone();
                    while let Some(next_due) = calculate_next_date(&current_due, interval) {
                        if next_due > *end_date {
                            break;
                        }
                        let mut nt = t.clone();
                        nt.id = Uuid::new_v4().to_string();
                        nt.due = Some(next_due.clone());
                        nt.status = TaskStatus::Todo;
                        nt.completed_at = None;
                        generated.push(nt);
                        current_due = next_due;
                    }
                }
            }
        }

        for nt in &generated {
            data.tasks.push(nt.clone());
        }

        t
    };
    state.save()?;

    let mut res = vec![updated];
    res.extend(generated);
    Ok(res)
}

pub fn toggle_task(state: &AppState, id: &str) -> Result<Task> {
    let updated = {
        let mut data = state.data.lock();
        let t = data.tasks.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(format!("task {id}")))?;

        let now = Utc::now().to_rfc3339();
        match t.status {
            TaskStatus::Todo => {
                t.status = TaskStatus::Done;
                t.completed_at = Some(now.clone());
            }
            TaskStatus::Done => {
                t.status = TaskStatus::Todo;
                t.completed_at = None;
            }
        }
        t.updated_at = now;
        t.clone()
    };

    state.save()?;
    Ok(updated)
}

/// フロントの useTaskStore.duplicateTask をバックエンドへ移したもの。
pub fn duplicate_task(state: &AppState, id: &str) -> Result<Task> {
    let src = state.data.lock().tasks.iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| Error::NotFound(format!("task {id}")))?;
    let mut nt = create_task(state, NewTask {
        title: format!("{} (コピー)", src.title),
        notes: Some(src.notes),
        priority: src.priority,
        due: src.due,
        project_id: src.project_id,
        tag_ids: Some(src.tag_ids),
        interval: src.interval,
        recurrence_id: src.recurrence_id,
        recurrence_end: src.recurrence_end,
    })?;
    if !src.subtasks.is_empty() {
        let mut data = state.data.lock();
        if let Some(t) = data.tasks.iter_mut().find(|t| t.id == nt.id) {
            t.subtasks = src.subtasks.iter()
                .map(|s| Subtask { id: Uuid::new_v4().to_string(), title: s.title.clone(), done: false })
                .collect();
            nt = t.clone();
        }
        drop(data);
        state.save()?;
    }
    Ok(nt)
}

/// コピーしておいたタスクを新しいタスクとして加える。タイトル・メモ・優先度・期日・プロジェクト・タグ・
/// サブタスクを引き継ぎ、完了・繰り返しは引き継がない（サブタスクは未完了に戻す）。
/// コピーした後に消えたプロジェクト・タグは外す
pub fn paste_task(state: &AppState, src: &Task) -> Result<Task> {
    let now = Utc::now().to_rfc3339();
    let task = {
        let mut data = state.data.lock();
        let project_id = src.project_id.clone().filter(|p| data.projects.iter().any(|x| &x.id == p));
        let tag_ids = src.tag_ids.iter().filter(|t| data.tags.iter().any(|x| &x.id == *t)).cloned().collect();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: src.title.clone(),
            notes: src.notes.clone(),
            status: TaskStatus::Todo,
            priority: src.priority.clone(),
            due: src.due.clone(),
            project_id,
            tag_ids,
            subtasks: src.subtasks.iter()
                .map(|s| Subtask { id: Uuid::new_v4().to_string(), title: s.title.clone(), done: false })
                .collect(),
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            interval: None,
            recurrence_id: None,
            recurrence_end: None,
        };
        data.tasks.push(task.clone());
        task
    };
    state.save()?;
    Ok(task)
}

pub fn delete_task(state: &AppState, id: &str) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        (dir, s)
    }

    #[test]
    fn paste_copies_content_but_not_completion_or_recurrence() {
        let (_d, s) = state();
        let mut src = s.data.lock().tasks[0].clone();
        src.status = TaskStatus::Done;
        src.completed_at = Some("2026-10-01T00:00:00Z".into());
        src.interval = Some(Interval::Weekly);
        src.recurrence_id = Some("r".into());
        src.subtasks = vec![Subtask { id: "s".into(), title: "sub".into(), done: true }];
        src.project_id = Some("gone".into());
        let before = s.data.lock().tasks.len();
        let t = paste_task(&s, &src).unwrap();
        assert_eq!(s.data.lock().tasks.len(), before + 1);
        assert_ne!(t.id, src.id);
        assert_eq!((t.title.as_str(), t.due.as_deref()), (src.title.as_str(), src.due.as_deref()));
        assert_eq!(t.status, TaskStatus::Todo);
        assert!(t.completed_at.is_none() && t.interval.is_none() && t.recurrence_id.is_none());
        assert_eq!(t.subtasks.len(), 1);
        assert!(!t.subtasks[0].done && t.subtasks[0].id != "s");
        // 消えたプロジェクトは外す
        assert!(t.project_id.is_none());
    }

    #[test]
    fn create_rejects_empty_title() {
        let (_d, s) = state();
        assert!(matches!(
            create_task(&s, NewTask { title: "  ".into(), ..Default::default() }),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn update_priority_none_clears_and_omitted_keeps() {
        let (_d, s) = state();
        let t = create_task(&s, NewTask { title: "a".into(), priority: Some(Priority::High), ..Default::default() }).unwrap();
        let r = update_task(&s, &t.id, TaskPatch { title: Some("b".into()), ..Default::default() }).unwrap();
        assert_eq!(r[0].priority, Some(Priority::High));
        let r = update_task(&s, &t.id, TaskPatch { priority: Some(None), ..Default::default() }).unwrap();
        assert_eq!(r[0].priority, None);
    }

    #[test]
    fn toggle_sets_and_clears_completed_at() {
        let (_d, s) = state();
        let t = create_task(&s, NewTask { title: "a".into(), ..Default::default() }).unwrap();
        assert!(toggle_task(&s, &t.id).unwrap().completed_at.is_some());
        assert!(toggle_task(&s, &t.id).unwrap().completed_at.is_none());
    }

    #[test]
    fn duplicate_copies_subtasks_as_undone() {
        let (_d, s) = state();
        let t = create_task(&s, NewTask { title: "a".into(), ..Default::default() }).unwrap();
        crate::commands::subtasks::add_subtask(&s, &t.id, "x").unwrap();
        let st = crate::commands::subtasks::add_subtask(&s, &t.id, "y").unwrap();
        crate::commands::subtasks::toggle_subtask(&s, &t.id, &st.id).unwrap();
        let d = duplicate_task(&s, &t.id).unwrap();
        assert_eq!(d.title, "a (コピー)");
        assert_eq!(d.subtasks.len(), 2);
        assert!(d.subtasks.iter().all(|x| !x.done));
    }
}
