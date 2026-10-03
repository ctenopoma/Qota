// 旧 Tauri 版 (コミット 8d92187 まで) の src-tauri/src/models.rs からの移植。時間計測 (estimatedMinutes / actualMinutes / startedAt) は持たない。
// 旧データに残っているそれらのキーは読み込み時に無視され、次の保存で消える。

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
    #[serde(default)]
    pub interval: Option<Interval>,
    #[serde(default)]
    pub recurrence_id: Option<String>,
    #[serde(default)]
    pub recurrence_end: Option<String>, // YYYY-MM-DD
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Todo,
    Done,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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
    /// 今日のタイムチャートの時間軸の範囲（時）。既定は 6〜24 時
    #[serde(default = "default_day_start_hour")]
    pub day_start_hour: u8,
    #[serde(default = "default_day_end_hour")]
    pub day_end_hour: u8,
}

fn default_day_start_hour() -> u8 { 6 }
fn default_day_end_hour() -> u8 { 24 }

/// 今日のタイムチャートに置いた計画の棒。日付ごとに残す（あとで計画と実績を分析するため、題名も控えておく）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanBlock {
    pub id: String,
    pub task_id: String,
    pub title: String,
    /// YYYY-MM-DD
    pub date: String,
    /// 0 時からの分（15 の倍数）
    pub start_min: u16,
    /// 長さ（分。15 の倍数）
    pub duration_min: u16,
}

/// 今日のタイムチャートの実績の棒。計画の棒（PlanBlock）と対にして、計画と現実のずれを後で分析する。
/// 何も記録しなかったときは「予定どおり」とみなし、日が替わるときに計画と同じ内容で残す
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActualBlock {
    pub id: String,
    pub task_id: String,
    pub title: String,
    /// YYYY-MM-DD
    pub date: String,
    /// 0 時からの分
    pub start_min: u16,
    pub end_min: u16,
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
    /// タイムチャートの計画。古いファイルには無いので、無ければ空
    #[serde(default)]
    pub plans: Vec<PlanBlock>,
    /// タイムチャートの実績。計画と同様、古いファイルには無いので、無ければ空
    #[serde(default)]
    pub actuals: Vec<ActualBlock>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Weekdays,
}

// Input DTOs
// IPC を経由しなくなったので serde ではなく Rust の構造体として直接組み立てる。
// `Option<Option<T>>` は「None = 変更なし / Some(None) = クリア」。

#[derive(Debug, Default)]
pub struct NewTask {
    pub title: String,
    pub notes: Option<String>,
    pub priority: Option<Priority>,
    pub due: Option<String>,
    pub project_id: Option<String>,
    pub tag_ids: Option<Vec<String>>,
    pub interval: Option<Interval>,
    pub recurrence_id: Option<String>,
    pub recurrence_end: Option<String>,
}

#[derive(Debug, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Option<Priority>>,
    pub due: Option<Option<String>>,
    pub project_id: Option<Option<String>>,
    pub tag_ids: Option<Vec<String>>,
    pub interval: Option<Option<Interval>>,
    pub recurrence_end: Option<Option<String>>,
    /// 繰り返しの 1 件を変えるとき、この回より後の回にも反映するか（false ならこの回だけ）。
    /// 頻度と終了日の変更は、いつでも以降の回を作り直す
    pub following: bool,
}

#[derive(Debug, Default)]
pub struct NewProject {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Default)]
pub struct SettingsPatch {
    pub mode: Option<Mode>,
    pub accent: Option<Accent>,
    pub density: Option<Density>,
    pub row_height: Option<u8>,
    pub week_start: Option<WeekStart>,
    pub day_start_hour: Option<u8>,
    pub day_end_hour: Option<u8>,
}
