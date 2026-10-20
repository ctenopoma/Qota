// 振り返り: 日・週・月の期間に完了したタスクと、タイムチャートの計画・実績の時間をプロジェクトごとにまとめる。
// 画面・Markdown のコピー・CSV の書き出しの元になる値をここで作る（Slint の型は src/view.rs で作る）。

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};

use crate::commands::plan::{self, ActualKind};
use crate::models::*;

/// 期間の単位。選んだものは ui-state.json に残す
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Day,
    #[default]
    Week,
    Month,
}

// 単位のセレクト (ui/review.slint) と同じ順
pub const UNITS: [Unit; 3] = [Unit::Day, Unit::Week, Unit::Month];

/// 期間（両端の日を含む）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Period {
    pub unit: Unit,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl Period {
    /// day を含む日・週・月。週の始まりは設定に合わせる
    pub fn containing(unit: Unit, day: NaiveDate, monday_first: bool) -> Self {
        let (start, end) = match unit {
            Unit::Day => (day, day),
            Unit::Week => {
                let back = if monday_first {
                    day.weekday().num_days_from_monday()
                } else {
                    day.weekday().num_days_from_sunday()
                };
                let start = day - Days::new(back as u64);
                (start, start + Days::new(6))
            }
            Unit::Month => {
                let start = day.with_day(1).expect("1st of month");
                (start, start + Months::new(1) - Days::new(1))
            }
        };
        Period { unit, start, end }
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        self.start <= d && d <= self.end
    }

    /// 見出しと Markdown に出す表記（2026/10/20（火）、2026/10/19（月）〜10/25（日）、2026年 10月）
    pub fn label(&self) -> String {
        match self.unit {
            Unit::Day => format!("{}（{}）", self.start.format("%Y/%m/%d"), weekday(self.start)),
            Unit::Week => {
                // 年をまたぐ週は、終わりにも年を付ける
                let end = if self.end.year() == self.start.year() { "%m/%d" } else { "%Y/%m/%d" };
                format!(
                    "{}（{}）〜{}（{}）",
                    self.start.format("%Y/%m/%d"),
                    weekday(self.start),
                    self.end.format(end),
                    weekday(self.end)
                )
            }
            Unit::Month => format!("{}年 {}月", self.start.year(), self.start.month()),
        }
    }

    /// CSV の保存ダイアログに出す名前
    pub fn file_name(&self) -> String {
        let stem = match self.unit {
            Unit::Day => self.start.format("%Y-%m-%d").to_string(),
            Unit::Week => format!("{}_{}", self.start.format("%Y-%m-%d"), self.end.format("%Y-%m-%d")),
            Unit::Month => self.start.format("%Y-%m").to_string(),
        };
        format!("qota-review-{stem}.csv")
    }
}

/// day を n 単位（日・週・月）ずらす。月は 1 日にそろえてから動かす（31 日から動かしても短い月を飛ばさない）
pub fn shift(unit: Unit, day: NaiveDate, n: i32) -> NaiveDate {
    match unit {
        Unit::Day => day + chrono::Duration::days(n as i64),
        Unit::Week => day + chrono::Duration::days(7 * n as i64),
        Unit::Month => {
            let first = day.with_day(1).expect("1st of month");
            let months = Months::new(n.unsigned_abs());
            if n >= 0 { first + months } else { first - months }
        }
    }
}

fn weekday(d: NaiveDate) -> &'static str {
    ["月", "火", "水", "木", "金", "土", "日"][d.weekday().num_days_from_monday() as usize]
}

/// 分を「1:05」の形に
pub fn hm(min: u32) -> String {
    format!("{}:{:02}", min / 60, min % 60)
}

/// 計画との差を「+0:15」「-1:00」「±0:00」の形に
pub fn signed_hm(min: i32) -> String {
    let sign = match min.signum() {
        1 => "+",
        -1 => "-",
        _ => "±",
    };
    format!("{sign}{}", hm(min.unsigned_abs()))
}

/// 0 時からの分を「09:30」の形に
fn clock(min: u16) -> String {
    format!("{:02}:{:02}", min / 60, min % 60)
}

/// 完了した日時（ローカル時刻）。完了していなければ None
fn completed_at(t: &Task) -> Option<DateTime<Local>> {
    if t.status != TaskStatus::Done {
        return None;
    }
    let ts = t.completed_at.as_deref()?;
    DateTime::parse_from_rfc3339(ts).ok().map(|d| d.with_timezone(&Local))
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// 回数と時間の合計
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sums {
    /// 完了した回数
    pub done: usize,
    /// 計画（分）
    pub plan: u32,
    /// 実績（分。進行中の分も含む）
    pub actual: u32,
    /// 実績 − 計画（分）。終わった計画の分だけで数える（進行中・まだ始まっていない計画は入れない）
    pub diff: i32,
    /// 差を数えた（終わった）計画がある
    pub finished: bool,
    /// まだ終わっていない計画（進行中・未着手）がある
    pub open: bool,
    /// 進行中の計画がある
    pub running: bool,
}

impl Sums {
    fn add(&mut self, o: &Sums) {
        self.done += o.done;
        self.plan += o.plan;
        self.actual += o.actual;
        self.diff += o.diff;
        self.finished |= o.finished;
        self.open |= o.open;
        self.running |= o.running;
    }

    /// 計画の棒 1 本と、その実績を足す
    fn add_block(&mut self, b: &PlanBlock, actual: Option<(ActualKind, u16, u16)>) {
        self.plan += b.duration_min as u32;
        match actual {
            None => self.open = true,
            Some((kind, s, e)) => {
                let len = e.saturating_sub(s) as u32;
                self.actual += len;
                if kind == ActualKind::Running {
                    self.running = true;
                    self.open = true;
                } else {
                    self.diff += len as i32 - b.duration_min as i32;
                    self.finished = true;
                }
            }
        }
    }
}

fn total<'a>(sums: impl Iterator<Item = &'a Sums>) -> Sums {
    let mut s = Sums::default();
    for x in sums {
        s.add(x);
    }
    s
}

/// 振り返りの 1 行。タスク 1 件（繰り返しは同じ系列をまとめて 1 行）
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// 詳細を開くタスク。消したタスクは None
    pub task_id: Option<String>,
    pub title: String,
    /// 期間の中で完了した日時（古い順）
    pub completed: Vec<DateTime<Local>>,
    pub sums: Sums,
    /// 並べ替え用: 期間の中で最初の計画（日, 開始）
    first_plan: Option<(NaiveDate, u16)>,
}

impl Row {
    pub fn deleted(&self) -> bool {
        self.task_id.is_none()
    }

    /// 完了の欄。1 回なら、日の振り返りでは時刻、週・月では日付。2 回以上なら回数
    pub fn done_label(&self, unit: Unit) -> String {
        match self.completed.as_slice() {
            [] => String::new(),
            [at] if unit == Unit::Day => at.format("%H:%M").to_string(),
            [at] => format!("{}/{}", at.month(), at.day()),
            many => format!("{} 回", many.len()),
        }
    }
}

/// プロジェクトごとのまとまり（サイドバーの順）。プロジェクトなし・消したタスクは末尾
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub name: String,
    /// プロジェクトの色（プロジェクトなし・消したタスクは None）
    pub color: Option<String>,
    pub rows: Vec<Row>,
}

impl Group {
    pub fn sums(&self) -> Sums {
        total(self.rows.iter().map(|r| &r.sums))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub period: Period,
    pub groups: Vec<Group>,
}

impl Review {
    pub fn sums(&self) -> Sums {
        let groups: Vec<Sums> = self.groups.iter().map(Group::sums).collect();
        total(groups.iter())
    }
}

/// 行をまとめる先
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Place {
    /// プロジェクト（None はプロジェクトなし）
    Project(Option<String>),
    /// 消したタスク（計画に残っている題名の控えで出す）
    Deleted,
}

fn task_place(t: &Task, data: &AppData) -> Place {
    // 無いプロジェクトを指していれば、プロジェクトなし
    Place::Project(t.project_id.clone().filter(|pid| data.projects.iter().any(|p| &p.id == pid)))
}

/// 同じ行にまとめるキー。繰り返しは系列で、ほかはタスクで
fn task_key(t: &Task) -> String {
    t.recurrence_id.clone().unwrap_or_else(|| t.id.clone())
}

#[derive(Default)]
struct Rows {
    list: Vec<(Place, Row)>,
    index: HashMap<(Place, String), usize>,
}

impl Rows {
    fn get(&mut self, place: Place, key: String, task_id: Option<&str>, title: &str) -> &mut Row {
        let i = *self.index.entry((place.clone(), key)).or_insert_with(|| {
            self.list.push((place, Row {
                task_id: task_id.map(str::to_string),
                title: title.to_string(),
                completed: vec![],
                sums: Sums::default(),
                first_plan: None,
            }));
            self.list.len() - 1
        });
        &mut self.list[i].1
    }
}

/// 期間に完了したタスクと、期間の計画・実績をまとめる。now_min（0 時からの分）は今日の計画の進み具合に使う
pub fn build(data: &AppData, period: Period, today: NaiveDate, now_min: u16) -> Review {
    let today = today.format("%Y-%m-%d").to_string();
    let mut rows = Rows::default();
    for t in &data.tasks {
        let Some(at) = completed_at(t).filter(|at| period.contains(at.date_naive())) else { continue };
        let row = rows.get(task_place(t, data), task_key(t), Some(&t.id), &t.title);
        row.completed.push(at);
        row.sums.done += 1;
    }
    for b in &data.plans {
        let Some(day) = parse_date(&b.date).filter(|d| period.contains(*d)) else { continue };
        let task = data.tasks.iter().find(|t| t.id == b.task_id);
        let row = match task {
            Some(t) => rows.get(task_place(t, data), task_key(t), Some(&t.id), &t.title),
            None => rows.get(Place::Deleted, b.task_id.clone(), None, &b.title),
        };
        let done = task.is_some_and(|t| t.status == TaskStatus::Done);
        row.sums.add_block(b, plan::actual_for(b, &data.actuals, done, &today, now_min));
        let here = (day, b.start_min);
        row.first_plan = Some(row.first_plan.map_or(here, |f| f.min(here)));
    }

    let places = data.projects.iter()
        .map(|p| (Place::Project(Some(p.id.clone())), p.name.clone(), Some(p.color.clone())))
        .chain([
            (Place::Project(None), "プロジェクトなし".to_string(), None),
            (Place::Deleted, "削除したタスク".to_string(), None),
        ]);
    let mut groups = vec![];
    for (place, name, color) in places {
        let mut group_rows: Vec<Row> = rows.list.iter()
            .filter(|(p, _)| *p == place)
            .map(|(_, r)| r.clone())
            .collect();
        if group_rows.is_empty() {
            continue;
        }
        for r in &mut group_rows {
            r.completed.sort();
        }
        // 完了したもの（最初に完了した順）、まだのもの（最初の計画の順）
        group_rows.sort_by_key(|r| (r.completed.is_empty(), r.completed.first().copied(), r.first_plan));
        groups.push(Group { name, color, rows: group_rows });
    }
    Review { period, groups }
}

/// 期間全体の 1 行（画面の上と Markdown の先頭）
fn summary_line(s: &Sums) -> String {
    let mut parts = vec![format!("完了 {} 件", s.done)];
    if s.plan > 0 {
        parts.push(format!("計画 {}", hm(s.plan)));
        parts.push(format!("実績 {}", hm(s.actual)));
        if s.finished {
            let note = if s.open { "（終わった計画のみ）" } else { "" };
            parts.push(format!("差 {}{note}", signed_hm(s.diff)));
        }
    }
    parts.join(" / ")
}

/// 日報・週報に貼る Markdown。完了したものは [x]、期間の中で終わらなかったものは [ ]
pub fn markdown(r: &Review) -> String {
    let mut out = format!("## 振り返り {}\n\n{}\n", r.period.label(), summary_line(&r.sums()));
    if r.groups.is_empty() {
        out.push_str("\nこの期間に完了したタスクや計画はありません\n");
        return out;
    }
    for g in &r.groups {
        out.push_str(&format!("\n### {}\n\n", g.name));
        for row in &g.rows {
            let mark = if row.completed.is_empty() { "[ ]" } else { "[x]" };
            let mut notes = vec![];
            match row.completed.len() {
                0 => {}
                1 => notes.push(format!("{} 完了", row.done_label(r.period.unit))),
                n => notes.push(format!("{n} 回完了")),
            }
            if row.sums.plan > 0 {
                notes.push(format!("計画 {}", hm(row.sums.plan)));
                if row.sums.actual > 0 || row.sums.running {
                    notes.push(format!("実績 {}", hm(row.sums.actual)));
                } else {
                    notes.push("未着手".into());
                }
            }
            if row.sums.running {
                notes.push("進行中".into());
            }
            let notes = if notes.is_empty() { String::new() } else { format!("（{}）", notes.join("・")) };
            out.push_str(&format!("- {mark} {}{notes}\n", row.title));
        }
    }
    out
}

/// CSV の 1 欄。カンマ・引用符・改行を含むときは引用符で囲む
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 計画と実績の分析用の CSV。計画の棒 1 本（日・タスクごと）につき 1 行で、計画と実績を並べる。
/// 計画の無い完了タスクも、完了した日の行として入れる。行は日付と時刻の順。BOM は保存するときに付ける
pub fn csv(data: &AppData, period: Period, today: NaiveDate, now_min: u16) -> String {
    let today = today.format("%Y-%m-%d").to_string();
    let project = |t: Option<&Task>| {
        t.and_then(|t| t.project_id.as_ref())
            .and_then(|pid| data.projects.iter().find(|p| &p.id == pid))
            .map(|p| p.name.clone())
            .unwrap_or_default()
    };
    let done_in_period = |t: &Task| completed_at(t).filter(|at| period.contains(at.date_naive()));
    let state = |t: Option<&Task>| match t {
        None => "削除",
        Some(t) if done_in_period(t).is_some() => "完了",
        Some(_) => "未完了",
    };
    let done_at = |t: Option<&Task>| {
        t.and_then(done_in_period).map(|at| at.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
    };

    let mut lines: Vec<((NaiveDate, u16), Vec<String>)> = vec![];
    for b in &data.plans {
        let Some(day) = parse_date(&b.date).filter(|d| period.contains(*d)) else { continue };
        let task = data.tasks.iter().find(|t| t.id == b.task_id);
        let done = task.is_some_and(|t| t.status == TaskStatus::Done);
        let actual = match plan::actual_for(b, &data.actuals, done, &today, now_min) {
            None => vec![String::new(), String::new(), String::new(), String::new(), "未着手".into()],
            Some((kind, s, e)) => {
                let len = e.saturating_sub(s) as i32;
                let running = kind == ActualKind::Running;
                let diff = if running { String::new() } else { (len - b.duration_min as i32).to_string() };
                vec![clock(s), clock(e), len.to_string(), diff, if running { "進行中" } else { "終了" }.into()]
            }
        };
        let mut row = vec![
            b.date.clone(),
            project(task),
            task.map(|t| t.title.clone()).unwrap_or_else(|| b.title.clone()),
            state(task).into(),
            done_at(task),
            clock(b.start_min),
            clock(b.start_min + b.duration_min),
            b.duration_min.to_string(),
        ];
        row.extend(actual);
        lines.push(((day, b.start_min), row));
    }
    for t in &data.tasks {
        let Some(at) = done_in_period(t) else { continue };
        let planned = data.plans.iter()
            .any(|b| b.task_id == t.id && parse_date(&b.date).is_some_and(|d| period.contains(d)));
        if planned {
            continue;
        }
        let mut row = vec![
            at.format("%Y-%m-%d").to_string(),
            project(Some(t)),
            t.title.clone(),
            "完了".into(),
            done_at(Some(t)),
        ];
        row.extend(std::iter::repeat_n(String::new(), 8));
        lines.push(((at.date_naive(), (at.hour() * 60 + at.minute()) as u16), row));
    }
    lines.sort_by_key(|(key, _)| *key);

    let mut out = String::from("日付,プロジェクト,タスク,状態,完了日時,計画開始,計画終了,計画(分),実績開始,実績終了,実績(分),差(分),実績の状態\r\n");
    for (_, row) in lines {
        out.push_str(&row.iter().map(|f| field(f)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    /// その日のその時刻（ローカル）に完了した、という completed_at
    fn at(day: &str, h: u32, m: u32) -> Option<String> {
        let t = d(day).and_hms_opt(h, m, 0).unwrap();
        Some(Local.from_local_datetime(&t).unwrap().to_rfc3339())
    }

    fn task(id: &str, title: &str, project: Option<&str>, done_at: Option<String>) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
            notes: String::new(),
            status: if done_at.is_some() { TaskStatus::Done } else { TaskStatus::Todo },
            priority: None,
            due: None,
            project_id: project.map(str::to_string),
            tag_ids: vec![],
            subtasks: vec![],
            created_at: String::new(),
            updated_at: String::new(),
            completed_at: done_at,
            interval: None,
            recurrence_id: None,
            recurrence_end: None,
        }
    }

    fn block(task: &str, title: &str, date: &str, start: u16, len: u16) -> PlanBlock {
        PlanBlock {
            id: format!("{task}-{date}"),
            task_id: task.into(),
            title: title.into(),
            date: date.into(),
            start_min: start,
            duration_min: len,
        }
    }

    fn actual(task: &str, date: &str, start: u16, end: u16) -> ActualBlock {
        ActualBlock {
            id: format!("a-{task}-{date}"),
            task_id: task.into(),
            title: String::new(),
            date: date.into(),
            start_min: start,
            end_min: end,
        }
    }

    /// 2026-10-19（月）〜25（日）の週。今日は 21 日（水）の 10:20
    fn sample() -> (AppData, Period) {
        let mut data = crate::seed::sample_data();
        let api = data.projects[0].id.clone(); // API リファクタ
        let site = data.projects[1].id.clone(); // マーケサイト
        let mut daily1 = task("c1", "日報", None, at("2026-10-19", 18, 0));
        let mut daily2 = task("c2", "日報", None, at("2026-10-20", 18, 0));
        let mut daily3 = task("c3", "日報", None, at("2026-10-26", 18, 0)); // 期間の外
        for t in [&mut daily1, &mut daily2, &mut daily3] {
            t.recurrence_id = Some("r1".into());
        }
        data.tasks = vec![
            task("a", "設計書", Some(&site), at("2026-10-20", 15, 0)),
            task("b", "レビュー", Some(&api), None),
            daily1,
            daily2,
            daily3,
            task("e", "先週の作業", Some(&api), at("2026-10-16", 12, 0)), // 期間の外
        ];
        data.plans = vec![
            block("a", "設計書", "2026-10-20", 9 * 60, 60),
            block("b", "レビュー", "2026-10-19", 13 * 60, 30), // 記録なし（前の日なので予定どおり）
            block("b", "レビュー", "2026-10-21", 10 * 60, 60), // 今日の最中
            block("gone", "消した作業", "2026-10-20", 11 * 60, 30),
            block("e", "先週の作業", "2026-10-16", 9 * 60, 30), // 期間の外
        ];
        data.actuals = vec![
            actual("a", "2026-10-20", 9 * 60, 10 * 60 + 30),
            actual("gone", "2026-10-20", 11 * 60, 11 * 60 + 30),
        ];
        (data, Period::containing(Unit::Week, d("2026-10-21"), true))
    }

    #[test]
    fn periods_follow_the_week_start_and_month_length() {
        // 2026-10-20 は火曜
        let mon = Period::containing(Unit::Week, d("2026-10-20"), true);
        assert_eq!((mon.start, mon.end), (d("2026-10-19"), d("2026-10-25")));
        let sun = Period::containing(Unit::Week, d("2026-10-20"), false);
        assert_eq!((sun.start, sun.end), (d("2026-10-18"), d("2026-10-24")));
        let feb = Period::containing(Unit::Month, d("2028-02-10"), true);
        assert_eq!((feb.start, feb.end), (d("2028-02-01"), d("2028-02-29")));

        assert_eq!(Period::containing(Unit::Day, d("2026-10-20"), true).label(), "2026/10/20（火）");
        assert_eq!(mon.label(), "2026/10/19（月）〜10/25（日）");
        assert_eq!(Period::containing(Unit::Week, d("2026-12-31"), true).label(), "2026/12/28（月）〜2027/01/03（日）");
        assert_eq!(feb.label(), "2028年 2月");
        assert_eq!(mon.file_name(), "qota-review-2026-10-19_2026-10-25.csv");
        assert_eq!(feb.file_name(), "qota-review-2028-02.csv");
    }

    #[test]
    fn shifting_a_month_does_not_skip_short_months() {
        assert_eq!(shift(Unit::Month, d("2026-01-31"), 1), d("2026-02-01"));
        assert_eq!(shift(Unit::Month, d("2026-03-31"), -1), d("2026-02-01"));
        assert_eq!(shift(Unit::Week, d("2026-10-20"), -1), d("2026-10-13"));
        assert_eq!(shift(Unit::Day, d("2026-10-01"), -1), d("2026-09-30"));
    }

    #[test]
    fn times_are_hours_and_minutes() {
        assert_eq!((hm(0), hm(125)), ("0:00".to_string(), "2:05".to_string()));
        assert_eq!(signed_hm(15), "+0:15");
        assert_eq!(signed_hm(-60), "-1:00");
        assert_eq!(signed_hm(0), "±0:00");
    }

    #[test]
    fn rows_are_grouped_by_project_with_series_merged() {
        let (data, period) = sample();
        let r = build(&data, period, d("2026-10-21"), 10 * 60 + 20);
        let names: Vec<&str> = r.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["API リファクタ", "マーケサイト", "プロジェクトなし", "削除したタスク"]);

        // 前の日の 30 分（予定どおり）と、今日の最中の 20 分。差は終わった計画の分だけ
        let review = &r.groups[0].rows[0];
        assert_eq!((review.title.as_str(), review.completed.len()), ("レビュー", 0));
        assert_eq!((review.sums.plan, review.sums.actual, review.sums.diff), (90, 50, 0));
        assert!(review.sums.running && review.sums.open);

        let design = &r.groups[1].rows[0];
        assert_eq!((design.sums.done, design.sums.plan, design.sums.actual, design.sums.diff), (1, 60, 90, 30));
        assert_eq!(design.done_label(Unit::Week), "10/20");
        assert_eq!(design.done_label(Unit::Day), "15:00");

        // 繰り返しは 1 行にまとめ、期間の外の回は数えない
        let daily = &r.groups[2].rows;
        assert_eq!(daily.len(), 1);
        assert_eq!((daily[0].sums.done, daily[0].done_label(Unit::Week)), (2, "2 回".to_string()));

        // 消したタスクは計画の題名の控えで出し、開けない
        let gone = &r.groups[3].rows[0];
        assert_eq!((gone.title.as_str(), gone.deleted()), ("消した作業", true));

        let s = r.sums();
        assert_eq!((s.done, s.plan, s.actual, s.diff), (3, 180, 170, 30));
    }

    #[test]
    fn markdown_lists_done_and_unfinished_by_project() {
        let (data, period) = sample();
        let r = build(&data, period, d("2026-10-21"), 10 * 60 + 20);
        assert_eq!(markdown(&r), "\
## 振り返り 2026/10/19（月）〜10/25（日）

完了 3 件 / 計画 3:00 / 実績 2:50 / 差 +0:30（終わった計画のみ）

### API リファクタ

- [ ] レビュー（計画 1:30・実績 0:50・進行中）

### マーケサイト

- [x] 設計書（10/20 完了・計画 1:00・実績 1:30）

### プロジェクトなし

- [x] 日報（2 回完了）

### 削除したタスク

- [ ] 消した作業（計画 0:30・実績 0:30）
");
        // まだ始まっていない今日の計画は「未着手」
        let early = build(&data, Period::containing(Unit::Day, d("2026-10-21"), true), d("2026-10-21"), 8 * 60);
        assert!(markdown(&early).contains("- [ ] レビュー（計画 1:00・未着手）\n"));

        let empty = build(&data, Period::containing(Unit::Day, d("2026-10-01"), true), d("2026-10-21"), 0);
        assert_eq!(markdown(&empty), "## 振り返り 2026/10/01（木）\n\n完了 0 件\n\nこの期間に完了したタスクや計画はありません\n");
    }

    #[test]
    fn csv_has_one_line_per_plan_and_per_unplanned_completion() {
        let (data, period) = sample();
        let lines: Vec<String> = csv(&data, period, d("2026-10-21"), 10 * 60 + 20)
            .split("\r\n")
            .map(str::to_string)
            .collect();
        assert_eq!(lines, [
            "日付,プロジェクト,タスク,状態,完了日時,計画開始,計画終了,計画(分),実績開始,実績終了,実績(分),差(分),実績の状態",
            "2026-10-19,API リファクタ,レビュー,未完了,,13:00,13:30,30,13:00,13:30,30,0,終了",
            "2026-10-19,,日報,完了,2026-10-19 18:00,,,,,,,,",
            "2026-10-20,マーケサイト,設計書,完了,2026-10-20 15:00,09:00,10:00,60,09:00,10:30,90,30,終了",
            "2026-10-20,,消した作業,削除,,11:00,11:30,30,11:00,11:30,30,0,終了",
            "2026-10-20,,日報,完了,2026-10-20 18:00,,,,,,,,",
            "2026-10-21,API リファクタ,レビュー,未完了,,10:00,11:00,60,10:00,10:20,20,,進行中",
            "",
        ]);
    }

    #[test]
    fn csv_quotes_commas_and_quotes() {
        assert_eq!(field("a,b"), "\"a,b\"");
        assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("plain"), "plain");
    }
}
