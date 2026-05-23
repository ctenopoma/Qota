use chrono::Utc;
use uuid::Uuid;
use crate::models::*;

pub fn sample_data() -> AppData {
    let now = Utc::now().to_rfc3339();
    
    let p_api = Project {
        id: Uuid::new_v4().to_string(),
        name: "API リファクタ".into(),
        color: "oklch(0.78 0.16 145)".into(),
        archived: false,
        created_at: now.clone(),
    };
    let p_marketing = Project {
        id: Uuid::new_v4().to_string(),
        name: "マーケサイト".into(),
        color: "oklch(0.6 0.2 250)".into(),
        archived: false,
        created_at: now.clone(),
    };
    
    let t_urgent = Tag {
        id: Uuid::new_v4().to_string(),
        name: "urgent".into(),
        created_at: now.clone(),
    };

    let task1 = Task {
        id: Uuid::new_v4().to_string(),
        title: "API ドキュメントの初稿を書く".into(),
        notes: "認可フローの図を含める".into(),
        status: TaskStatus::Todo,
        priority: Some(Priority::High),
        due: Some("2026-05-08".into()),
        project_id: Some(p_api.id.clone()),
        tag_ids: vec![t_urgent.id.clone()],
        subtasks: vec![
            Subtask { id: Uuid::new_v4().to_string(), title: "概要".into(), done: false },
            Subtask { id: Uuid::new_v4().to_string(), title: "エンドポイント一覧".into(), done: false },
        ],
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
        interval: None,
        recurrence_id: None,
        recurrence_end: None,
        estimated_minutes: None,
        actual_minutes: None,
        started_at: None,
    };

    AppData {
        schema_version: 1,
        tasks: vec![task1],
        projects: vec![p_api, p_marketing],
        tags: vec![t_urgent],
        settings: Settings {
            mode: Mode::Dark,
            accent: Accent::Green,
            density: Density::Comfortable,
            row_height: 12,
            week_start: WeekStart::Mon,
        },
    }
}
