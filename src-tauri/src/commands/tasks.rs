use chrono::{Datelike, Days, NaiveDate, Utc, Weekday};
use tauri::State;
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

#[tauri::command]
pub fn create_task(state: State<'_, AppState>, input: NewTask) -> Result<Task> {
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
        estimated_minutes: input.estimated_minutes,
        actual_minutes: None,
        started_at: None,
        interval: input.interval.clone(),
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

#[tauri::command]
pub fn update_task(state: State<'_, AppState>, id: String, patch: TaskPatch) -> Result<Vec<Task>> {
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
            || patch.recurrence_end.is_some()
            || patch.estimated_minutes.is_some();

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
            if let Some(v) = patch.estimated_minutes { t.estimated_minutes = v; }
            if let Some(v) = patch.actual_minutes { t.actual_minutes = v; }
            if let Some(v) = patch.started_at { t.started_at = v; }
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
                        nt.started_at = None;
                        nt.actual_minutes = None;
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

#[tauri::command]
pub fn toggle_task(state: State<'_, AppState>, id: String) -> Result<Vec<Task>> {
    let updated = {
        let mut data = state.data.lock();
        let idx = data.tasks.iter().position(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(format!("task {id}")))?;
        
        let now = Utc::now().to_rfc3339();
        
        // toggle logic
        let status = &data.tasks[idx].status;
        match status {
            TaskStatus::Todo => {
                data.tasks[idx].status = TaskStatus::Done;
                data.tasks[idx].completed_at = Some(now.clone());
                // If timer was running, calculate actual_minutes from started_at
                if let Some(ref started) = data.tasks[idx].started_at {
                    if let Ok(start_time) = chrono::DateTime::parse_from_rfc3339(started) {
                        let elapsed = Utc::now().signed_duration_since(start_time);
                        let mins = elapsed.num_minutes().max(1) as u32;
                        // Add to existing actual_minutes or set new
                        let existing = data.tasks[idx].actual_minutes.unwrap_or(0);
                        data.tasks[idx].actual_minutes = Some(existing + mins);
                    }
                    data.tasks[idx].started_at = None;
                }
            }
            TaskStatus::Done => {
                data.tasks[idx].status = TaskStatus::Todo;
                data.tasks[idx].completed_at = None;
                data.tasks[idx].started_at = None;
            }
        }
        data.tasks[idx].updated_at = now;
        
        vec![data.tasks[idx].clone()]
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
