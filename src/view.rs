// AppData から Slint に渡す表示用の値を組み立てる。
// 旧 Tauri 版 (コミット 8d92187 まで) の src/lib/date.ts と src/pages/{Today,Inbox,ProjectDetail}.tsx のフィルタ・並び替えをここへ移した。

use chrono::{Datelike, Local, NaiveDate};
use slint::{Color, Model, ModelRc, SharedString, VecModel};
use std::collections::HashSet;
use std::rc::Rc;

use crate::models::*;

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
    let done = t.status == TaskStatus::Done;
    let Some(d) = t.due.as_deref().and_then(parse_date) else { return Some(7) };
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
            // sortByPriority(sortByDue(...))：期日順に並べてから優先度で安定ソート
            sort_by_due(&mut items);
            items.sort_by_key(|t| match t.priority {
                Some(Priority::High) => 0,
                Some(Priority::Medium) => 1,
                Some(Priority::Low) => 2,
                None => 3,
            });
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

/// プロジェクトごとの列。タスクはデータの順（元アプリと同じ）、見出しの右は「完了/全体」
pub fn project_columns(data: &AppData, expanded: &HashSet<String>) -> Vec<crate::Column> {
    data.projects.iter()
        .map(|p| {
            let items: Vec<&Task> = data.tasks.iter().filter(|t| t.project_id.as_deref() == Some(p.id.as_str())).collect();
            let done = items.iter().filter(|t| t.status == TaskStatus::Done).count();
            column(&p.id, &p.name, format!("{done}/{}", items.len()), Some(&p.color), &items, data, expanded)
        })
        .collect()
}

/// タグごとの列。タスクは期日順、見出しの右は件数
pub fn tag_columns(data: &AppData, expanded: &HashSet<String>) -> Vec<crate::Column> {
    data.tags.iter()
        .map(|tag| {
            let mut items: Vec<&Task> = data.tasks.iter().filter(|t| t.tag_ids.contains(&tag.id)).collect();
            sort_by_due(&mut items);
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
        let day_tasks: Vec<&Task> = data.tasks.iter().filter(|t| t.due.as_deref() == Some(iso.as_str())).collect();
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

/// その日のタスク（日付を押したときの一覧）
pub fn day_tasks(data: &AppData, year: i32, month: i32, day: i32, expanded: &HashSet<String>) -> Vec<crate::TaskItem> {
    let Some(iso) = iso(year, month, day) else { return vec![] };
    data.tasks.iter()
        .filter(|t| t.due.as_deref() == Some(iso.as_str()))
        .map(|t| task_item(t, data, expanded))
        .collect()
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
}
