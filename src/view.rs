// AppData から Slint に渡す表示用の値を組み立てる。
// 旧 Tauri 版 (コミット 8d92187 まで) の src/lib/date.ts と src/pages/{Today,Inbox,ProjectDetail}.tsx のフィルタ・並び替えをここへ移した。

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use slint::{Color, Model, ModelRc, SharedString, VecModel};
use std::collections::HashSet;
use std::rc::Rc;

use crate::models::*;
use crate::review;

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn parse_date(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

pub fn iso(y: i32, m: i32, d: i32) -> Option<String> {
    NaiveDate::from_ymd_opt(y, m as u32, d as u32).map(|d| d.format("%Y-%m-%d").to_string())
}

pub fn is_today(due: Option<&str>) -> bool {
    due.and_then(parse_date) == Some(today())
}

pub fn is_overdue(due: Option<&str>) -> bool {
    due.and_then(parse_date).is_some_and(|d| d < today())
}

/// date.ts の formatDue
pub fn format_due(due: Option<&str>) -> String {
    let Some(d) = due.and_then(parse_date) else { return String::new() };
    let t = today();
    if d == t {
        "今日".into()
    } else if Some(d) == t.succ_opt() {
        "明日".into()
    } else if d < t && (t - d).num_days() <= 99 {
        // 過ぎた期日は、何日前かのほうが「5/8」より分かる
        format!("{}日前", (t - d).num_days())
    } else {
        format!("{}/{}", d.month(), d.day())
    }
}

/// date.ts の sortByDue（期日なしは末尾、安定ソート）
pub fn sort_by_due(tasks: &mut [&Task]) {
    tasks.sort_by(|a, b| match (&a.due, &b.due) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(y),
    });
}

/// 完了済みを末尾へ寄せる。安定ソートなので、未完了に戻せば元の位置に戻る
pub fn done_last(tasks: &mut [&Task]) {
    tasks.sort_by_key(|t| t.status == TaskStatus::Done);
}

/// 優先度の高い順（なしは末尾）
fn priority_rank(p: Option<Priority>) -> u8 {
    match p {
        Some(Priority::High) => 0,
        Some(Priority::Medium) => 1,
        Some(Priority::Low) => 2,
        None => 3,
    }
}

/// 今日の一覧の並び順。選んだものは ui-state.json に残す
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Due,
    Priority,
    Project,
    Created,
}

// 並び順のセレクト (ui/task-table.slint の sort-options) と同じ順
pub const SORT_KEYS: [SortKey; 4] = [SortKey::Due, SortKey::Priority, SortKey::Project, SortKey::Created];

/// 優先度順・プロジェクト順は、同じもののあいだを期日順にする。
/// プロジェクト順はサイドバーの並びで、プロジェクトなしは末尾。追加順は古いものから
pub fn sort_tasks(tasks: &mut [&Task], key: SortKey, projects: &[Project]) {
    match key {
        SortKey::Due => sort_by_due(tasks),
        SortKey::Priority => {
            sort_by_due(tasks);
            tasks.sort_by_key(|t| priority_rank(t.priority));
        }
        SortKey::Project => {
            sort_by_due(tasks);
            tasks.sort_by_key(|t| {
                t.project_id.as_ref()
                    .and_then(|pid| projects.iter().position(|p| &p.id == pid))
                    .unwrap_or(usize::MAX)
            });
        }
        SortKey::Created => tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
    }
}

pub fn priority_to_int(p: Option<Priority>) -> i32 {
    match p {
        None => 0,
        Some(Priority::High) => 1,
        Some(Priority::Medium) => 2,
        Some(Priority::Low) => 3,
    }
}

pub fn int_to_priority(i: i32) -> Option<Priority> {
    match i {
        1 => Some(Priority::High),
        2 => Some(Priority::Medium),
        3 => Some(Priority::Low),
        _ => None,
    }
}

// TaskDetail の「頻度」セレクトの並び順
pub const INTERVALS: [Interval; 5] = [
    Interval::Daily,
    Interval::Weekdays,
    Interval::Weekly,
    Interval::Monthly,
    Interval::Yearly,
];

/// プロジェクト色 ("oklch(L C H)" か "#rrggbb") を sRGB に変換する
pub fn parse_color(s: &str) -> Color {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            if hex.len() == 6 {
                return Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8);
            }
        }
    }
    if let Some(body) = s.strip_prefix("oklch(").and_then(|r| r.strip_suffix(')')) {
        let v: Vec<f64> = body
            .split(|c: char| c.is_whitespace() || c == '/')
            .filter(|x| !x.is_empty())
            .filter_map(|x| x.trim_end_matches('%').parse().ok())
            .collect();
        if v.len() >= 3 {
            let (r, g, b) = oklch_to_srgb(v[0], v[1], v[2]);
            return Color::from_rgb_u8(r, g, b);
        }
    }
    Color::from_rgb_u8(0x88, 0x88, 0x88)
}

fn oklch_to_srgb(l: f64, c: f64, h: f64) -> (u8, u8, u8) {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    let lin = [
        4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
        -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
        -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_,
    ];
    let enc = |x: f64| {
        let v = if x <= 0.0031308 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    (enc(lin[0]), enc(lin[1]), enc(lin[2]))
}

fn ymd(due: Option<&str>) -> (i32, i32, i32) {
    match due.and_then(parse_date) {
        Some(d) => (d.year(), d.month() as i32, d.day() as i32),
        None => (0, 0, 0),
    }
}

/// <input type="date"> の ja-JP 表示 (2026/05/23)
fn format_full(iso: Option<&str>) -> String {
    iso.and_then(parse_date)
        .map(|d| d.format("%Y/%m/%d").to_string())
        .unwrap_or_default()
}

/// 日付ピッカーのマス。year 年 month 月を含む週を、前後の月の日も含めて並べる
pub fn month_days(year: i32, month: i32, monday_first: bool) -> Vec<crate::CalendarDay> {
    let Some(first) = NaiveDate::from_ymd_opt(year, month as u32, 1) else { return vec![] };
    let offset = if monday_first {
        first.weekday().num_days_from_monday()
    } else {
        first.weekday().num_days_from_sunday()
    };
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month as u32 + 1, 1)
    };
    let days_in_month = next_month.map(|n| (n - first).num_days() as u32).unwrap_or(31);
    let cells = (offset + days_in_month).div_ceil(7) * 7;
    let start = first - chrono::Days::new(offset as u64);
    (0..cells)
        .map(|i| {
            let d = start + chrono::Days::new(i as u64);
            crate::CalendarDay {
                year: d.year(),
                month: d.month() as i32,
                day: d.day() as i32,
                in_month: d.month() as i32 == month,
            }
        })
        .collect()
}

pub fn task_item(t: &Task, data: &AppData, expanded: &HashSet<String>) -> crate::TaskItem {
    let project = t.project_id.as_ref().and_then(|pid| data.projects.iter().find(|p| &p.id == pid));
    let tags: Vec<SharedString> = data.tags.iter()
        .filter(|tag| t.tag_ids.contains(&tag.id))
        .map(|tag| tag.name.as_str().into())
        .collect();
    let subtasks: Vec<crate::SubtaskItem> = t.subtasks.iter().map(subtask_item).collect();
    let (y, m, d) = ymd(t.due.as_deref());
    crate::TaskItem {
        id: t.id.as_str().into(),
        title: t.title.as_str().into(),
        done: t.status == TaskStatus::Done,
        priority: priority_to_int(t.priority),
        due_label: format_due(t.due.as_deref()).into(),
        overdue: is_overdue(t.due.as_deref()),
        due_year: y,
        due_month: m,
        due_day: d,
        project_name: project.map(|p| p.name.as_str()).unwrap_or("").into(),
        project_color: project.map(|p| parse_color(&p.color)).unwrap_or_default(),
        tags: ModelRc::new(VecModel::from(tags)),
        repeating: t.interval.is_some(),
        sub_done: t.subtasks.iter().filter(|s| s.done).count() as i32,
        sub_total: t.subtasks.len() as i32,
        subtasks: ModelRc::new(VecModel::from(subtasks)),
        expanded: expanded.contains(&t.id),
    }
}

pub fn subtask_item(s: &Subtask) -> crate::SubtaskItem {
    crate::SubtaskItem { id: s.id.as_str().into(), title: s.title.as_str().into(), done: s.done }
}

pub fn detail_data(t: &Task, data: &AppData) -> crate::TaskDetailData {
    let (y, m, d) = ymd(t.due.as_deref());
    let (ey, em, ed) = ymd(t.recurrence_end.as_deref());
    let project_index = t.project_id.as_ref()
        .and_then(|pid| data.projects.iter().position(|p| &p.id == pid))
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    crate::TaskDetailData {
        id: t.id.as_str().into(),
        title: t.title.as_str().into(),
        notes: t.notes.as_str().into(),
        done: t.status == TaskStatus::Done,
        priority: priority_to_int(t.priority),
        due_label: format_full(t.due.as_deref()).into(),
        due_year: y,
        due_month: m,
        due_day: d,
        project_index,
        repeating: t.interval.is_some(),
        interval_index: t.interval
            .and_then(|iv| INTERVALS.iter().position(|x| *x == iv))
            .unwrap_or(0) as i32,
        recurrence_end_label: format_full(t.recurrence_end.as_deref()).into(),
        recurrence_end_year: ey,
        recurrence_end_month: em,
        recurrence_end_day: ed,
    }
}

/// 行数が同じなら差分だけ set_row_data する。行の再生成を避けてポップアップやフォーカスを保つ。
pub fn sync<T: Clone + PartialEq + 'static>(model: &Rc<VecModel<T>>, rows: Vec<T>) {
    let old = model.row_count();
    for (i, row) in rows.iter().enumerate() {
        if i < old {
            if model.row_data(i).as_ref() != Some(row) {
                model.set_row_data(i, row.clone());
            }
        } else {
            model.push(row.clone());
        }
    }
    for i in (rows.len()..old).rev() {
        model.remove(i);
    }
}

// ───── 今後（カンバン）───── 旧 Tauri 版の Upcoming.tsx と date.ts の判定をそのまま移す

/// date.ts の isThisWeek。今後のビューは週の始まりを渡していないので月曜始まり
fn is_this_week(d: NaiveDate, today: NaiveDate) -> bool {
    let start = today - chrono::Days::new(today.weekday().num_days_from_monday() as u64);
    let end = start + chrono::Days::new(7);
    d >= start && d < end
}

fn is_this_month(d: NaiveDate, today: NaiveDate) -> bool {
    d.year() == today.year() && d.month() == today.month()
}

fn is_next_month(d: NaiveDate, today: NaiveDate) -> bool {
    let (y, m) = if today.month() == 12 { (today.year() + 1, 1) } else { (today.year(), today.month() + 1) };
    d.year() == y && d.month() == m
}

/// 今後の列（見出し, 判定）。完了済みは「期限切れ」以外の列に残る（元アプリと同じ）
pub fn upcoming_bucket(t: &Task, today: NaiveDate) -> Option<usize> {
    bucket_for(t.due.as_deref().and_then(parse_date), t.status == TaskStatus::Done, today)
}

fn bucket_for(due: Option<NaiveDate>, done: bool, today: NaiveDate) -> Option<usize> {
    let Some(d) = due else { return Some(7) };
    let overdue = d < today;
    let is_today = d == today;
    let tomorrow = Some(d) == today.succ_opt();
    let week = is_this_week(d, today);
    let month = is_this_month(d, today);
    let next_month = is_next_month(d, today);
    if overdue && !done {
        Some(0)
    } else if is_today {
        Some(1)
    } else if tomorrow {
        Some(2)
    } else if week && !overdue {
        Some(3)
    } else if month && !week && !overdue {
        Some(4)
    } else if next_month && !week {
        Some(5)
    } else if !month && !next_month && !week && !overdue {
        Some(6)
    } else {
        None
    }
}

/// カードを今後の列に落としたときの期日。その列に入る最初の日（今日から数える）。
/// 「記載なし」は期日を外すので Some(None)、「期限切れ」や日が見つからない列は None
pub fn drop_due_for_column(index: usize, today: NaiveDate) -> Option<Option<NaiveDate>> {
    match index {
        0 => None,
        7 => Some(None),
        _ => (0..800u64)
            .map(|i| today + chrono::Days::new(i))
            .find(|d| bucket_for(Some(*d), false, today) == Some(index))
            .map(Some),
    }
}

pub const UPCOMING_TITLES: [&str; 8] = ["期限切れ", "今日", "明日", "今週", "今月", "来月", "それ以降", "記載なし"];

pub fn upcoming_columns(data: &AppData, expanded: &HashSet<String>, today: NaiveDate) -> Vec<crate::Column> {
    let mut buckets: Vec<Vec<&Task>> = vec![Vec::new(); 8];
    for t in &data.tasks {
        if let Some(i) = upcoming_bucket(t, today) {
            buckets[i].push(t);
        }
    }
    UPCOMING_TITLES.iter().zip(buckets).enumerate()
        .map(|(i, (title, mut items))| {
            // sortByPriority(sortByDue(...))：期日順に並べてから優先度で安定ソート。完了済みは列の末尾
            sort_by_due(&mut items);
            items.sort_by_key(|t| priority_rank(t.priority));
            done_last(&mut items);
            column(&format!("upcoming-{i}"), title, items.len().to_string(), None, &items, data, expanded)
        })
        .collect()
}

fn column(
    id: &str,
    title: &str,
    sub: String,
    color: Option<&str>,
    items: &[&Task],
    data: &AppData,
    expanded: &HashSet<String>,
) -> crate::Column {
    let tasks: Vec<crate::TaskItem> = items.iter().map(|t| task_item(t, data, expanded)).collect();
    crate::Column {
        id: id.into(),
        title: title.into(),
        sub: sub.into(),
        color: color.map(parse_color).unwrap_or_default(),
        has_color: color.is_some(),
        color_key: color.unwrap_or("").into(),
        tasks: ModelRc::new(VecModel::from(tasks)),
    }
}

// ───── プロジェクト一覧・タグ ─────

/// プロジェクトごとの列。完了済みは出さず、タスクはデータの順。見出しの右は完了済みも含めた「完了/全体」
pub fn project_columns(data: &AppData, expanded: &HashSet<String>) -> Vec<crate::Column> {
    data.projects.iter()
        .map(|p| {
            let (done, open): (Vec<&Task>, Vec<&Task>) = data.tasks.iter()
                .filter(|t| t.project_id.as_deref() == Some(p.id.as_str()))
                .partition(|t| t.status == TaskStatus::Done);
            let sub = format!("{}/{}", done.len(), done.len() + open.len());
            column(&p.id, &p.name, sub, Some(&p.color), &open, data, expanded)
        })
        .collect()
}

/// タグごとの列。タスクは期日順で完了済みは末尾、見出しの右は件数
pub fn tag_columns(data: &AppData, expanded: &HashSet<String>) -> Vec<crate::Column> {
    data.tags.iter()
        .map(|tag| {
            let mut items: Vec<&Task> = data.tasks.iter().filter(|t| t.tag_ids.contains(&tag.id)).collect();
            sort_by_due(&mut items);
            done_last(&mut items);
            column(&tag.id, &format!("#{}", tag.name), items.len().to_string(), None, &items, data, expanded)
        })
        .collect()
}

// ───── カレンダー ───── 旧 Tauri 版の Calendar.tsx

/// 月初めの前を空白で埋め、その月の日を並べる（月末の後ろは埋めない。元アプリと同じ）
pub fn calendar_cells(
    data: &AppData,
    year: i32,
    month: i32,
    monday_first: bool,
    expanded: &HashSet<String>,
    today: NaiveDate,
) -> Vec<crate::CalCell> {
    let Some(first) = NaiveDate::from_ymd_opt(year, month as u32, 1) else { return vec![] };
    let offset = if monday_first {
        first.weekday().num_days_from_monday()
    } else {
        first.weekday().num_days_from_sunday()
    };
    let blank = crate::CalCell {
        year,
        month,
        day: 0,
        today: false,
        tasks: ModelRc::default(),
        more: 0,
    };
    let mut cells: Vec<crate::CalCell> = (0..offset).map(|_| blank.clone()).collect();
    let mut d = first;
    while d.month() == first.month() {
        let iso = d.format("%Y-%m-%d").to_string();
        // 完了済みは末尾に寄せ、マスに出しきれないときは未完了を先に見せる
        let mut day_tasks: Vec<&Task> = data.tasks.iter().filter(|t| t.due.as_deref() == Some(iso.as_str())).collect();
        done_last(&mut day_tasks);
        let shown: Vec<crate::TaskItem> = day_tasks.iter().take(2).map(|t| task_item(t, data, expanded)).collect();
        cells.push(crate::CalCell {
            year,
            month,
            day: d.day() as i32,
            today: d == today,
            tasks: ModelRc::new(VecModel::from(shown)),
            more: day_tasks.len().saturating_sub(2) as i32,
        });
        d = d.succ_opt().expect("date overflow");
    }
    cells
}

/// その日のタスク（日付を押したときの一覧）。完了済みは末尾
pub fn day_tasks(data: &AppData, year: i32, month: i32, day: i32, expanded: &HashSet<String>) -> Vec<crate::TaskItem> {
    let Some(iso) = iso(year, month, day) else { return vec![] };
    let mut items: Vec<&Task> = data.tasks.iter().filter(|t| t.due.as_deref() == Some(iso.as_str())).collect();
    done_last(&mut items);
    items.iter().map(|t| task_item(t, data, expanded)).collect()
}

/// 通知パネル: 期限切れ（未完了）と今日（未完了）
pub fn notifications(data: &AppData, expanded: &HashSet<String>) -> (Vec<crate::TaskItem>, Vec<crate::TaskItem>) {
    let open = |t: &&Task| t.status != TaskStatus::Done;
    let overdue = data.tasks.iter().filter(open).filter(|t| is_overdue(t.due.as_deref()))
        .map(|t| task_item(t, data, expanded)).collect();
    let today = data.tasks.iter().filter(open).filter(|t| is_today(t.due.as_deref()))
        .map(|t| task_item(t, data, expanded)).collect();
    (overdue, today)
}

// ───── 振り返り ─────

/// 差の色分け（1 計画より長い / -1 短い / 0 同じか、比べられる計画が無い）
fn diff_sign(s: &review::Sums) -> i32 {
    if s.finished { s.diff.signum() } else { 0 }
}

pub fn review_summary(r: &review::Review) -> crate::ReviewSummary {
    let s = r.sums();
    crate::ReviewSummary {
        done: s.done as i32,
        plan: review::hm(s.plan).into(),
        actual: review::hm(s.actual).into(),
        diff: if s.finished { review::signed_hm(s.diff) } else { "–".into() }.into(),
        diff_sign: diff_sign(&s),
        diff_note: if s.finished && s.open { "終わった計画のみ" } else { "" }.into(),
    }
}

pub fn review_groups(r: &review::Review) -> Vec<crate::ReviewGroup> {
    r.groups.iter()
        .map(|g| {
            let total = g.sums();
            let mut sub = vec![];
            if total.done > 0 {
                sub.push(format!("完了 {} 件", total.done));
            }
            if total.plan > 0 {
                sub.push(format!("実績 {}", review::hm(total.actual)));
            }
            let rows: Vec<crate::ReviewRow> = g.rows.iter()
                .map(|row| {
                    let s = &row.sums;
                    let actual = if s.plan == 0 {
                        String::new()
                    } else if s.running {
                        format!("{}…", review::hm(s.actual))
                    } else if s.actual == 0 {
                        "–".into()
                    } else {
                        review::hm(s.actual)
                    };
                    // 差は、その行の計画がすべて終わってから出す
                    let compared = s.finished && !s.open;
                    crate::ReviewRow {
                        task_id: row.task_id.as_deref().unwrap_or("").into(),
                        title: row.title.as_str().into(),
                        done: !row.completed.is_empty(),
                        deleted: row.deleted(),
                        done_label: row.done_label(r.period.unit).into(),
                        plan: if s.plan > 0 { review::hm(s.plan) } else { String::new() }.into(),
                        actual: actual.into(),
                        diff: if compared { review::signed_hm(s.diff) } else { String::new() }.into(),
                        diff_sign: if compared { diff_sign(s) } else { 0 },
                    }
                })
                .collect();
            crate::ReviewGroup {
                name: g.name.as_str().into(),
                color: g.color.as_deref().map(parse_color).unwrap_or_default(),
                has_color: g.color.is_some(),
                sub: sub.join(" ・ ").into(),
                rows: ModelRc::new(VecModel::from(rows)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_oklch_matches_theme_conversion() {
        // theme.slint の accent (dark) と同じ値になること
        assert_eq!(parse_color("oklch(0.78 0.16 145)"), Color::from_rgb_u8(0x6e, 0xd2, 0x74));
        assert_eq!(parse_color("#3b82f6"), Color::from_rgb_u8(0x3b, 0x82, 0xf6));
    }

    #[test]
    fn month_days_covers_whole_weeks() {
        // 2026-10-01 は木曜。日曜始まりなら 9/27 から、月曜始まりなら 9/28 から並ぶ
        let sun = month_days(2026, 10, false);
        assert_eq!(sun.len(), 35);
        assert_eq!((sun[0].month, sun[0].day), (9, 27));
        assert_eq!((sun[4].month, sun[4].day, sun[4].in_month), (10, 1, true));
        assert_eq!((sun[34].month, sun[34].day), (10, 31));

        let mon = month_days(2026, 10, true);
        assert_eq!((mon[0].month, mon[0].day), (9, 28));
        assert_eq!(mon.len() % 7, 0);

        // 12 月から翌年 1 月をまたぐ
        let dec = month_days(2026, 12, false);
        let last = dec.last().unwrap();
        assert_eq!((last.year, last.month, last.day), (2027, 1, 2));
    }

    #[test]
    fn detail_dates_use_ja_format() {
        assert_eq!(format_full(Some("2026-05-08")), "2026/05/08");
        assert_eq!(format_full(None), "");
    }

    fn task_due(due: Option<&str>, done: bool) -> Task {
        let mut t = crate::seed::sample_data().tasks.remove(0);
        t.due = due.map(String::from);
        t.status = if done { TaskStatus::Done } else { TaskStatus::Todo };
        t
    }

    #[test]
    fn upcoming_buckets_follow_original_rules() {
        // 2026-10-01 は木曜。今週 = 9/28(月)〜10/4(日)
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let b = |due: Option<&str>, done: bool| upcoming_bucket(&task_due(due, done), today);
        assert_eq!(b(Some("2026-09-30"), false), Some(0)); // 期限切れ
        assert_eq!(b(Some("2026-09-30"), true), None); // 期限切れの完了済みはどこにも出ない
        assert_eq!(b(Some("2026-10-01"), true), Some(1)); // 今日（完了済みも出す）
        assert_eq!(b(Some("2026-10-02"), false), Some(2)); // 明日
        assert_eq!(b(Some("2026-10-04"), false), Some(3)); // 今週
        assert_eq!(b(Some("2026-10-20"), false), Some(4)); // 今月
        assert_eq!(b(Some("2026-11-03"), false), Some(5)); // 来月
        assert_eq!(b(Some("2027-01-10"), false), Some(6)); // それ以降
        assert_eq!(b(None, false), Some(7)); // 記載なし
    }

    #[test]
    fn project_columns_hide_done_tasks_but_count_them() {
        let mut data = crate::seed::sample_data();
        let pid = data.projects[0].id.clone();
        let mut open = task_due(None, false);
        open.id = "open".into();
        open.project_id = Some(pid.clone());
        let mut done = task_due(None, true);
        done.id = "done".into();
        done.project_id = Some(pid);
        data.tasks = vec![done, open];
        let cols = project_columns(&data, &HashSet::new());
        assert_eq!(cols[0].sub, "1/2");
        let ids: Vec<String> = cols[0].tasks.iter().map(|t| t.id.to_string()).collect();
        assert_eq!(ids, ["open"]);
    }

    #[test]
    fn upcoming_columns_put_done_tasks_last_and_restore_when_undone() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let mut data = crate::seed::sample_data();
        data.tasks = ["a", "b", "c"].iter()
            .map(|id| {
                let mut t = task_due(Some("2026-10-01"), false);
                t.id = (*id).into();
                t.priority = None;
                t
            })
            .collect();
        let today_ids = |data: &AppData| -> String {
            let cols = upcoming_columns(data, &HashSet::new(), today);
            cols[1].tasks.iter().map(|t| t.id.to_string()).collect()
        };
        assert_eq!(today_ids(&data), "abc");
        data.tasks[0].status = TaskStatus::Done;
        assert_eq!(today_ids(&data), "bca");
        data.tasks[0].status = TaskStatus::Todo;
        assert_eq!(today_ids(&data), "abc");
    }

    #[test]
    fn sort_tasks_breaks_ties_by_due() {
        let data = crate::seed::sample_data();
        let (p0, p1) = (data.projects[0].id.clone(), data.projects[1].id.clone());
        let task = |id: &str, due: &str, pri: Option<Priority>, project: Option<&str>, created: &str| {
            let mut t = task_due(Some(due), false);
            t.id = id.into();
            t.priority = pri;
            t.project_id = project.map(String::from);
            t.created_at = created.into();
            t
        };
        let tasks = [
            task("a", "2026-10-02", None, Some(&p1), "2026-09-01T00:00:00+09:00"),
            task("b", "2026-10-02", Some(Priority::High), None, "2026-09-03T00:00:00+09:00"),
            task("c", "2026-09-30", Some(Priority::Low), Some(&p0), "2026-09-02T00:00:00+09:00"),
            task("d", "2026-09-29", None, Some(&p1), "2026-09-04T00:00:00+09:00"),
        ];
        let order = |key| {
            let mut v: Vec<&Task> = tasks.iter().collect();
            sort_tasks(&mut v, key, &data.projects);
            v.iter().map(|t| t.id.as_str()).collect::<String>()
        };
        assert_eq!(order(SortKey::Due), "dcab");
        assert_eq!(order(SortKey::Priority), "bcda"); // 優先度なしの d と a は期日順
        assert_eq!(order(SortKey::Project), "cdab"); // プロジェクトなしの b は末尾
        assert_eq!(order(SortKey::Created), "acbd");
    }

    #[test]
    fn calendar_starts_with_blanks_and_marks_today() {
        let data = crate::seed::sample_data();
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let cells = calendar_cells(&data, 2026, 10, false, &HashSet::new(), today);
        // 日曜始まりなら木曜の 10/1 の前に空白 4 つ
        assert_eq!(cells.iter().take_while(|c| c.day == 0).count(), 4);
        assert_eq!(cells.len(), 4 + 31);
        assert!(cells[4].today && cells[4].day == 1);
        let mon = calendar_cells(&data, 2026, 10, true, &HashSet::new(), today);
        assert_eq!(mon.iter().take_while(|c| c.day == 0).count(), 3);
    }

    #[test]
    fn dropping_on_a_column_picks_the_first_day_that_belongs_to_it() {
        // 2026-10-07（水）、2026-10-31（土）、2026-12-31（木）など、週・月の境目も確かめる
        for today in ["2026-10-07", "2026-10-31", "2026-12-31", "2026-10-04"] {
            let today = NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap();
            assert_eq!(drop_due_for_column(0, today), None);
            assert_eq!(drop_due_for_column(7, today), Some(None));
            for index in 1..=6 {
                // 週末なら「今週」、月末なら「今月」に入る日が無いことがある
                let Some(d) = drop_due_for_column(index, today).flatten() else {
                    assert!(matches!(index, 3 | 4), "列 {index} に入る日が無い（{today}）");
                    continue;
                };
                assert_eq!(bucket_for(Some(d), false, today), Some(index), "{today} 列 {index}");
                // 前の日はその列に入らない（= 最初の日）
                if d > today {
                    assert_ne!(bucket_for(Some(d.pred_opt().unwrap()), false, today), Some(index), "{today} 列 {index}");
                }
            }
        }
    }
}
