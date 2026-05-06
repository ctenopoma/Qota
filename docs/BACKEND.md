# Qota バックエンド設計書

> Tauri v2 + Rust。データストアは JSON ファイル単体。Windows ローカル専用、シングルユーザー。

---

## 1. 全体方針

- **データの単一ソース** — `qota.json` 一枚にすべての状態を保持する。
- **同期書き込み** — 各コマンドは「メモリ更新 → ディスク書き出し」を一気に行う。
- **アトミック書き込み** — `qota.json.tmp` に書いてから rename する（中断時の破損防止）。
- **排他制御** — `parking_lot::Mutex<AppState>` で全コマンドを排他。
- **不変な ID** — `uuid::Uuid::new_v4().to_string()` を使い、ID はバックエンド側で生成。
- **タイムスタンプ** — `chrono::Utc::now().to_rfc3339()` を使う（フロントは ISO 8601 をそのまま受ける）。
- **エラー** — 独自 `Error` enum を `serde::Serialize` 化し、`Result<T, Error>` を返す。

---

## 2. ファイル配置

```
src-tauri/
├── Cargo.toml
├── build.rs
├── tauri.conf.json
├── capabilities/
│   └── default.json
└── src/
    ├── main.rs           ← qota_lib::run() を呼ぶだけ
    ├── lib.rs            ← Tauri ビルダー、invoke_handler 登録
    ├── error.rs          ← Error 型
    ├── models.rs         ← Task / Project / Tag / Settings / AppData の構造体
    ├── store.rs          ← AppState、JSON ファイルの load/save
    ├── seed.rs           ← サンプルデータ
    └── commands/
        ├── mod.rs
        ├── tasks.rs      ← create_task / update_task / toggle_task / delete_task
        ├── subtasks.rs   ← add_subtask / toggle_subtask / delete_subtask
        ├── projects.rs   ← create_project / update_project / delete_project
        ├── tags.rs       ← create_tag / delete_tag
        ├── settings.rs   ← update_settings
        ├── search.rs     ← search
        └── io.rs         ← load_all / export_json / import_json / reset_sample_data
```

---

## 3. データファイルの場所

Tauri v2 の `app.path().app_data_dir()` を使う。

| OS | パス |
|---|---|
| Windows | `%APPDATA%\com.example.qota\qota.json` |

例：`C:\Users\<user>\AppData\Roaming\com.example.qota\qota.json`

初回起動時にディレクトリが無ければ作成し、`qota.json` が無ければ `seed.rs` のサンプルデータを書き出す。

---

## 4. JSON スキーマ (`qota.json`)

```jsonc
{
  "schemaVersion": 1,
  "tasks": [
    {
      "id": "9b1c…",
      "title": "API ドキュメントの初稿を書く",
      "notes": "認可フローの図を含める",
      "status": "todo",                   // "todo" | "done"
      "priority": "high",                  // "high" | "medium" | "low" | null
      "due": "2026-05-08",                 // YYYY-MM-DD | null
      "projectId": "p_…",                  // null = 受信箱
      "tagIds": ["t_…"],
      "subtasks": [
        { "id": "s_…", "title": "概要", "done": false }
      ],
      "createdAt": "2026-05-06T09:00:00Z", // RFC3339
      "updatedAt": "2026-05-06T09:00:00Z",
      "completedAt": null
    }
  ],
  "projects": [
    {
      "id": "p_…",
      "name": "API リファクタ",
      "color": "oklch(0.78 0.16 145)",
      "archived": false,
      "createdAt": "2026-05-01T00:00:00Z"
    }
  ],
  "tags": [
    { "id": "t_…", "name": "urgent", "createdAt": "2026-05-01T00:00:00Z" }
  ],
  "settings": {
    "mode": "dark",
    "accent": "green",
    "density": "comfortable",
    "rowHeight": 12,
    "weekStart": "mon"
  }
}
```

### スキーマバージョン管理

- `schemaVersion` が将来上がった場合は `store.rs::migrate()` で v1 → v2 のマイグレーションを実装する。
- 読み込み時に `schemaVersion` が未定義 / 期待より大きい場合はエラー。

---

## 5. Rust 型定義 (`models.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub status: TaskStatus,
    pub priority: Option<Priority>,
    pub due: Option<String>,           // YYYY-MM-DD
    pub project_id: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
    pub created_at: String,            // RFC3339
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Todo,
    Done,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub title: String,
    pub done: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub archived: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mode: Mode,
    pub accent: Accent,
    pub density: Density,
    pub row_height: u8,
    pub week_start: WeekStart,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode { Dark, Light }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Accent { Green, Blue, Amber, Violet }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Density { Compact, Comfortable, Spacious }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart { Mon, Sun }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub schema_version: u32,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub settings: Settings,
}
```

> **重要**: `#[serde(rename_all = "camelCase")]` でフロントの TypeScript と命名規則を揃える。

### 入力 DTO

```rust
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub tag_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskPatch {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub status: Option<TaskStatus>,
    /// `null` を渡すと "優先度なし" にする。フィールド省略は変更なし。
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub priority: Option<Option<Priority>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub due: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub project_id: Option<Option<String>>,
    pub tag_ids: Option<Vec<String>>,
}

// "省略 vs null" を区別する helper（Option<Option<T>> パターン）
fn deserialize_optional_field<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where T: Deserialize<'de>, D: serde::Deserializer<'de>
{ Ok(Some(Option::deserialize(d)?)) }
```

`Option<Option<T>>` を使うのは「キーが無い = 変更なし」と「キーが `null` = クリア」を区別するため。

---

## 6. ストア (`store.rs`)

```rust
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct AppState {
    pub data: Mutex<AppData>,
    pub path: PathBuf,
}

impl AppState {
    pub fn load_or_init(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let path = app_data_dir.join("qota.json");
        let data = if path.exists() {
            let bytes = std::fs::read(&path)?;
            let data: AppData = serde_json::from_slice(&bytes)?;
            // migrate(&mut data)?;
            data
        } else {
            let seed = crate::seed::sample_data();
            atomic_write(&path, &seed)?;
            seed
        };
        Ok(Self { data: Mutex::new(data), path })
    }

    pub fn save(&self) -> Result<()> {
        let guard = self.data.lock();
        atomic_write(&self.path, &*guard)
    }
}

fn atomic_write(path: &PathBuf, data: &AppData) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(data)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
```

`AppState` は `tauri::Builder::default().manage(state)` で登録し、各コマンドで `State<'_, AppState>` として受け取る。

---

## 7. エラー型 (`error.rs`)

```rust
use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]   Io(#[from] std::io::Error),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
    #[error("not found: {0}")] NotFound(String),
    #[error("invalid input: {0}")] InvalidInput(String),
    #[error("schema version unsupported: {0}")] UnsupportedSchema(u32),
    #[error("dialog cancelled")] Cancelled,
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
```

`Result<T, Error>` を返すコマンドは、フロント側では `try { ... } catch (e) { ... }` で文字列として受ける。

---

## 8. コマンド一覧

すべて `#[tauri::command]` で実装し、`lib.rs` の `tauri::generate_handler![...]` に列挙する。

| コマンド | 入力 | 出力 | 内容 |
|---|---|---|---|
| `load_all` | – | `AppData` | 起動時にスナップショットを返す |
| `create_task` | `input: NewTask` | `Task` | id, createdAt, updatedAt を生成 |
| `update_task` | `id: String`, `patch: TaskPatch` | `Task` | 部分更新。null と省略を区別 |
| `toggle_task` | `id: String` | `Task` | status 反転、`completedAt` 設定/解除 |
| `delete_task` | `id: String` | `()` | – |
| `add_subtask` | `taskId: String`, `title: String` | `Subtask` | – |
| `toggle_subtask` | `taskId: String`, `subtaskId: String` | `Subtask` | done 反転 |
| `delete_subtask` | `taskId: String`, `subtaskId: String` | `()` | – |
| `create_project` | `input: NewProject` | `Project` | デフォルト color はパレットからローテーション |
| `update_project` | `id`, `patch: ProjectPatch` | `Project` | – |
| `delete_project` | `id` | `()` | 関連タスクの `projectId` を null にする |
| `create_tag` | `name: String` | `Tag` | 重複は InvalidInput |
| `delete_tag` | `id: String` | `()` | 関連タスクの `tagIds` から削除 |
| `update_settings` | `patch: Partial<Settings>` | `Settings` | – |
| `search` | `q: String` | `Vec<Task>` | title / notes に対する大文字小文字無視の部分一致 |
| `export_json` | – | `String` | tauri-plugin-dialog で保存先を選ばせ、書き出し、パスを返す |
| `import_json` | – | `AppData` | dialog で選び、検証、`AppState` に流し込む |
| `reset_sample_data` | – | `AppData` | seed に戻す |

### 命名規則

- コマンド名は **snake_case**（Rust 標準）。フロント側の invoke もそのまま `'create_task'` で呼ぶ。
- 引数名（invoke の第2引数 object のキー）は **camelCase**。Tauri の serde 設定により Rust 側 snake_case のフィールドにマップされる。
  - 例: `invoke('toggle_subtask', { taskId, subtaskId })` ↔ Rust 側 `task_id: String, subtask_id: String`

---

## 9. コマンド実装例

### `commands/tasks.rs`

```rust
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
```

### `commands/io.rs`

```rust
use tauri::State;
use tauri_plugin_dialog::{DialogExt, FilePath};
use crate::error::{Error, Result};
use crate::models::AppData;
use crate::store::AppState;

#[tauri::command]
pub async fn load_all(state: State<'_, AppState>) -> Result<AppData> {
    Ok(state.data.lock().clone())
}

#[tauri::command]
pub async fn export_json(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String> {
    // ダイアログは非同期 (channel 経由で結果を受け取る)
    let path: Option<FilePath> = app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name("qota-export.json")
        .blocking_save_file();
    let Some(path) = path else { return Err(Error::Cancelled); };
    let path_buf = path.into_path().map_err(|e| Error::InvalidInput(e.to_string()))?;
    let data = state.data.lock().clone();
    let json = serde_json::to_vec_pretty(&data)?;
    std::fs::write(&path_buf, json)?;
    Ok(path_buf.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn import_json(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<AppData> {
    let path: Option<FilePath> = app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file();
    let Some(path) = path else { return Err(Error::Cancelled); };
    let path_buf = path.into_path().map_err(|e| Error::InvalidInput(e.to_string()))?;
    let bytes = std::fs::read(&path_buf)?;
    let new_data: AppData = serde_json::from_slice(&bytes)?;
    if new_data.schema_version != 1 {
        return Err(Error::UnsupportedSchema(new_data.schema_version));
    }
    {
        let mut data = state.data.lock();
        *data = new_data.clone();
    }
    state.save()?;
    Ok(new_data)
}

#[tauri::command]
pub async fn reset_sample_data(state: State<'_, AppState>) -> Result<AppData> {
    let seed = crate::seed::sample_data();
    {
        let mut data = state.data.lock();
        *data = seed.clone();
    }
    state.save()?;
    Ok(seed)
}
```

### `commands/search.rs`

```rust
#[tauri::command]
pub fn search(state: State<'_, AppState>, q: String) -> Result<Vec<Task>> {
    let needle = q.to_lowercase();
    if needle.trim().is_empty() {
        return Ok(vec![]);
    }
    let data = state.data.lock();
    let hits: Vec<Task> = data.tasks.iter()
        .filter(|t|
            t.title.to_lowercase().contains(&needle)
            || t.notes.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    Ok(hits)
}
```

---

## 10. `lib.rs` (起動)

```rust
use tauri::Manager;

mod commands;
mod error;
mod models;
mod seed;
mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()
                .expect("failed to resolve app_data_dir");
            let state = store::AppState::load_or_init(dir)
                .expect("failed to load app state");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::io::load_all,
            commands::io::export_json,
            commands::io::import_json,
            commands::io::reset_sample_data,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::toggle_task,
            commands::tasks::delete_task,
            commands::subtasks::add_subtask,
            commands::subtasks::toggle_subtask,
            commands::subtasks::delete_subtask,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::tags::create_tag,
            commands::tags::delete_tag,
            commands::settings::update_settings,
            commands::search::search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 11. capabilities (`capabilities/default.json`)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default"
  ]
}
```

> JSON ファイル自体は `app_data_dir` 配下なので `fs:default` の scope に含まれる。フロントから直接 `fs` プラグインで読み書きする必要は無く、すべて `#[tauri::command]` 経由で行う。`fs` はインポート / エクスポート時のダイアログ選択ファイルアクセスのみ使う。

---

## 12. シードデータ (`seed.rs`)

サンプルとして次を含めると UI が空でなく見える：

- プロジェクト 3 件（API リファクタ / マーケサイト / 個人）
- タグ 4 件（urgent / waiting / idea / bug）
- タスク 12〜15 件（うち 3 件は期日切れ、3 件は今日、4 件は今後、3 件は受信箱、1 件は完了済み）
- サブタスクを持つタスク 1〜2 件

実装例:

```rust
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
    // ... (同様にタスク等を構築)
    AppData {
        schema_version: 1,
        tasks: vec![/* ... */],
        projects: vec![p_api /* ... */],
        tags: vec![/* ... */],
        settings: Settings {
            mode: Mode::Dark,
            accent: Accent::Green,
            density: Density::Comfortable,
            row_height: 12,
            week_start: WeekStart::Mon,
        },
    }
}
```

---

## 13. テスト指針

- `#[cfg(test)]` で `commands::tasks::create_task` 等を直接呼ぶ単体テスト。
- `tempfile` クレートで `app_data_dir` 相当の一時ディレクトリを使う。
- `cargo test --manifest-path src-tauri/Cargo.toml`。

最低限カバーすべきケース:

| 関数 | ケース |
|---|---|
| `create_task` | 正常 / 空タイトルで InvalidInput |
| `update_task` | priority に null 指定でクリア / フィールド省略で変更なし |
| `toggle_task` | done 化で `completed_at` セット、戻し時に null |
| `delete_project` | 関連タスクの projectId が null になる |
| `delete_tag` | 関連タスクの tagIds から消える |
| `import_json` | schemaVersion mismatch で UnsupportedSchema |
| `atomic_write` | tmp 失敗時に元ファイルが壊れない |

---

## 14. 実装順チェックリスト

1. [ ] `Cargo.toml` 依存追加（`chrono`, `uuid`, `parking_lot`, `thiserror`, plugins）
2. [ ] `models.rs` 作成
3. [ ] `error.rs` 作成
4. [ ] `store.rs` で AppState + atomic_write
5. [ ] `seed.rs` でサンプルデータ
6. [ ] `lib.rs::run` の `setup` で AppState を manage
7. [ ] `commands/tasks.rs` から順に実装
8. [ ] `invoke_handler!` に登録
9. [ ] `capabilities/default.json` に dialog/fs 権限追加
10. [ ] `cargo tauri dev` でフロントから `load_all` が返ることを確認
11. [ ] 単体テスト追加

---

## 15. フロントとの整合チェックリスト

- [ ] Rust 構造体のフィールドはすべて `#[serde(rename_all = "camelCase")]`
- [ ] enum は `#[serde(rename_all = "lowercase")]` で TypeScript リテラルと一致
- [ ] invoke の引数キーは camelCase（`taskId` 等）
- [ ] `null` を許す Option フィールドは `Option<T>`、`null` 渡しでクリアしたい patch は `Option<Option<T>>`
- [ ] 戻り値はフロント `src/types.ts` と 1:1
- [ ] エラーは文字列としてフロント側で `catch (e)` で受ける
