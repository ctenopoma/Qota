// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod history;
mod instance;
mod models;
mod seed;
mod store;
mod view;
mod window_state;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDate};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};

use commands::{io, projects, search, settings, subtasks, tags, tasks};
use error::Error;
use models::*;
use store::AppState;

slint::include_modules!();

struct App {
    state: AppState,
    ui: slint::Weak<AppWindow>,
    // サブタスクを展開している行の task id
    expanded: RefCell<HashSet<String>>,
    detail_id: RefCell<Option<String>>,
    active: Rc<VecModel<TaskItem>>,
    done: Rc<VecModel<TaskItem>>,
    projects: Rc<VecModel<NavProject>>,
    detail_subtasks: Rc<VecModel<SubtaskItem>>,
    detail_tags: Rc<VecModel<TagChip>>,
    commands: Rc<VecModel<Command>>,
    // 今後・プロジェクト一覧・タグの列、カレンダーのマス、日付の一覧、検索結果、通知
    columns: Rc<VecModel<Column>>,
    cal_cells: Rc<VecModel<CalCell>>,
    day_tasks: Rc<VecModel<TaskItem>>,
    search_results: Rc<VecModel<TaskItem>>,
    notif_overdue: Rc<VecModel<TaskItem>>,
    notif_today: Rc<VecModel<TaskItem>>,
    // カレンダーで表示中の年月と、一覧を開いている日
    cal_month: Cell<(i32, i32)>,
    open_day: Cell<Option<(i32, i32, i32)>>,
    search_query: RefCell<String>,
    toast_timer: Timer,
    // 確認ダイアログで「はい」が押されたら実行する操作
    pending: RefCell<Option<Pending>>,
    // Ctrl+Z / Ctrl+Y とトーストの「元に戻す」
    undo_stack: RefCell<history::UndoStack>,
    // マウスの戻る・進むボタン / Alt+← → 用の画面履歴 (view, project-id)
    nav: RefCell<history::NavStack<Route>>,
    // 詳細パネルで入力中（未保存）の内容
    drafts: RefCell<Drafts>,
    // 起動したまま日付が変わったら描き直すための、今の日付と 1 分ごとのタイマー
    today: Cell<NaiveDate>,
    day_timer: Timer,
    // Ctrl+C でコピーしたタスク
    clip: RefCell<Option<Clip>>,
}

/// コピーしたタスクと、そのとき OS のクリップボードに置いた文字（タイトル）。
/// 貼り付けるのは、クリップボードがその文字のままのときだけ（ほかで文字をコピーしたら貼らない）
struct Clip {
    task: Task,
    // クリップボードに書けなかったときは None。アプリ内のコピーだけで判断する
    text: Option<String>,
}

enum Pending {
    ResetData,
}

type Route = (SharedString, SharedString);

#[derive(Default)]
struct Drafts {
    title: Option<(String, String)>,
    notes: Option<(String, String)>,
    subtask: Option<(String, String, String)>,
}

impl App {
    fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("window is gone")
    }

    /// 操作のたびに呼ぶ。AppData から今のビューに必要なものを組み直して UI に流す
    fn refresh(&self) {
        let ui = self.ui();
        let data = self.state.data.lock();
        let route = ui.get_view();
        let pid = ui.get_project_id();

        let (title, mut visible): (String, Vec<&Task>) = match route.as_str() {
            "today" => (
                "今日".into(),
                data.tasks.iter()
                    .filter(|t| view::is_today(t.due.as_deref())
                        || (view::is_overdue(t.due.as_deref()) && t.status != TaskStatus::Done))
                    .collect(),
            ),
            "inbox" => (
                "受信箱".into(),
                data.tasks.iter().filter(|t| t.project_id.is_none()).collect(),
            ),
            "project" => (
                data.projects.iter()
                    .find(|p| p.id == pid.as_str())
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "プロジェクトが見つかりません".into()),
                data.tasks.iter().filter(|t| t.project_id.as_deref() == Some(pid.as_str())).collect(),
            ),
            _ => (String::new(), vec![]),
        };
        if route == "today" {
            let key = view::SORT_KEYS.get(ui.get_today_sort() as usize).copied().unwrap_or_default();
            view::sort_tasks(&mut visible, key, &data.projects);
        } else {
            view::sort_by_due(&mut visible);
        }

        let expanded = self.expanded.borrow();
        let (done, active): (Vec<&Task>, Vec<&Task>) =
            visible.iter().partition(|t| t.status == TaskStatus::Done);
        let subtitle = match route.as_str() {
            "inbox" => "プロジェクト未割当".to_string(),
            "project" => format!("{} todo", active.len()),
            _ => String::new(),
        };
        ui.set_list_title(title.into());
        ui.set_list_subtitle(subtitle.into());
        view::sync(&self.active, active.iter().map(|t| view::task_item(t, &data, &expanded)).collect());
        view::sync(&self.done, done.iter().map(|t| view::task_item(t, &data, &expanded)).collect());

        view::sync(
            &self.projects,
            data.projects.iter()
                .filter(|p| !p.archived)
                .map(|p| NavProject {
                    id: p.id.as_str().into(),
                    name: p.name.as_str().into(),
                    color: view::parse_color(&p.color),
                })
                .collect(),
        );
        ui.set_overdue_count(
            data.tasks.iter()
                .filter(|t| view::is_overdue(t.due.as_deref()) && t.status != TaskStatus::Done)
                .count() as i32,
        );

        let mut options: Vec<SharedString> = vec!["受信箱".into()];
        options.extend(data.projects.iter().map(|p| SharedString::from(p.name.as_str())));
        ui.set_project_options(ModelRc::new(VecModel::from(options)));

        // 詳細パネル
        let detail = self.detail_id.borrow().as_ref()
            .and_then(|id| data.tasks.iter().find(|t| &t.id == id));
        match detail {
            Some(t) => {
                ui.set_detail(view::detail_data(t, &data));
                view::sync(&self.detail_subtasks, t.subtasks.iter().map(view::subtask_item).collect());
                view::sync(
                    &self.detail_tags,
                    data.tags.iter()
                        .map(|tag| TagChip {
                            id: tag.id.as_str().into(),
                            name: tag.name.as_str().into(),
                            on: t.tag_ids.contains(&tag.id),
                        })
                        .collect(),
                );
            }
            None => ui.set_detail_open(false),
        }

        ui.global::<Theme>().set_dark(data.settings.mode == Mode::Dark);
        if route == "settings" {
            ui.set_latest_backup(store::latest_backup_date(self.state.dir()).unwrap_or_default().into());
        }

        // 表示中の画面の分だけ組み立てる
        let today = self.today.get();
        match route.as_str() {
            "upcoming" => view::sync(&self.columns, view::upcoming_columns(&data, &expanded, today)),
            "projects" => view::sync(&self.columns, view::project_columns(&data, &expanded)),
            "tags" => view::sync(&self.columns, view::tag_columns(&data, &expanded)),
            "calendar" => {
                let (y, m) = self.cal_month.get();
                let monday_first = data.settings.week_start == WeekStart::Mon;
                ui.set_cal_title(format!("{y}年 {m}月").into());
                ui.set_cal_monday_first(monday_first);
                view::sync(&self.cal_cells, view::calendar_cells(&data, y, m, monday_first, &expanded, today));
                if let Some((y, m, d)) = self.open_day.get() {
                    ui.set_day_title(format!("{y}年 {m}月 {d}日").into());
                    view::sync(&self.day_tasks, view::day_tasks(&data, y, m, d, &expanded));
                }
            }
            _ => {}
        }
        let (overdue, due_today) = view::notifications(&data, &expanded);
        view::sync(&self.notif_overdue, overdue);
        view::sync(&self.notif_today, due_today);
        drop(data);
        if ui.get_search_open() {
            let q = self.search_query.borrow().clone();
            self.run_search(&q);
        }
    }

    fn run_search(&self, q: &str) {
        let ui = self.ui();
        ui.set_search_has_query(!q.trim().is_empty());
        let hits = search::search(&self.state, q);
        let data = self.state.data.lock();
        let expanded = self.expanded.borrow();
        view::sync(&self.search_results, hits.iter().map(|t| view::task_item(t, &data, &expanded)).collect());
    }

    /// データを変える操作はすべてここを通す。直前の状態を取り消し用に積み、結果を画面に反映する。
    /// label は取り消しのトーストに出す説明（「『abc』の削除」など）。操作の前のデータから作る
    fn mutate<T>(
        &self,
        label: impl FnOnce(&AppData) -> String,
        f: impl FnOnce(&AppState) -> error::Result<T>,
    ) -> Option<T> {
        let before = self.state.data.lock().clone();
        let r = f(&self.state);
        if *self.state.data.lock() != before {
            let label = label(&before);
            self.undo_stack.borrow_mut().record(before, label);
            // トーストの「元に戻す」は直前の操作を指すので、新しい操作が入ったら消す
            self.ui().set_toast_action(SharedString::new());
        }
        let out = match r {
            Ok(v) => Some(v),
            Err(e) => {
                self.report(e);
                None
            }
        };
        self.refresh();
        out
    }

    /// 取り消しの対象にしない操作（表示モードの切り替え）の結果を反映する
    fn apply<T>(&self, r: error::Result<T>) {
        if let Err(e) = r {
            self.report(e);
        }
        self.refresh();
    }

    fn report(&self, e: Error) {
        if !matches!(e, Error::Cancelled) {
            eprintln!("error: {e}");
            self.toast(format!("エラー: {e}"), false, true);
        }
    }

    /// 画面下部に通知を出す。undoable なら「元に戻す」ボタンが付く。新しい通知は前のものを置き換える
    fn toast(&self, msg: String, undoable: bool, error: bool) {
        let ui = self.ui();
        ui.set_toast_action(if undoable { "元に戻す".into() } else { SharedString::new() });
        ui.set_toast_error(error);
        ui.set_toast_text(shorten_middle(&msg, 64).into());
        let weak = self.ui.clone();
        let secs = if undoable { 5 } else { 3 };
        self.toast_timer.start(slint::TimerMode::SingleShot, Duration::from_secs(secs), move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_toast_text(SharedString::new());
                ui.set_toast_action(SharedString::new());
            }
        });
    }

    /// Ctrl+Z / Ctrl+Y / トーストの「元に戻す」。
    /// 入力中の下書きは捨てない（入力欄に見えている内容は、閉じるときに戻した後のデータへ保存する）
    fn step_history(&self, undo: bool) {
        let current = self.state.data.lock().clone();
        let settings = current.settings.clone();
        let target = {
            let mut h = self.undo_stack.borrow_mut();
            if undo { h.undo(current) } else { h.redo(current) }
        };
        let Some((mut target, label)) = target else {
            self.toast(if undo { "取り消せる操作はありません" } else { "やり直せる操作はありません" }.into(), false, false);
            return;
        };
        // 表示モードは取り消しの対象外なので、今の設定を引き継ぐ
        target.settings = settings;
        *self.state.data.lock() = target;
        let saved = self.state.save();
        self.toast(format!("{label}を{}", if undo { "取り消しました" } else { "やり直しました" }), false, false);
        if let Err(e) = saved {
            self.report(e);
        }
        self.refresh();
    }

    /// 1 分ごとに呼ぶ。日付が変わっていたら「今日」を差し替えて描き直す（入力中の内容や開いているパネルは触らない）
    fn check_date(&self) {
        let t = view::today();
        if t != self.today.get() {
            self.today.set(t);
            set_bridge_today(&self.ui(), t);
            self.refresh();
        }
    }

    fn route(&self) -> Route {
        let ui = self.ui();
        (ui.get_view(), ui.get_project_id())
    }

    fn route_exists(&self, (view, pid): &Route) -> bool {
        view != "project" || self.state.data.lock().projects.iter().any(|p| p.id == pid.as_str())
    }

    fn show_route(&self, (view, pid): Route) {
        let ui = self.ui();
        ui.set_view(view);
        ui.set_project_id(pid);
        ui.global::<Bridge>().set_hover_task(SharedString::new());
        self.refresh();
    }

    /// 画面を移る。戻るボタンのために今の画面を履歴に積む
    fn go(&self, view: SharedString, pid: SharedString) {
        let current = self.route();
        if current == (view.clone(), pid.clone()) {
            return;
        }
        self.nav.borrow_mut().visit(current);
        self.show_route((view, pid));
    }

    /// マウスの戻るボタン / Alt+←。パネルやダイアログが開いていれば、先にそれを閉じる
    fn nav_back(&self) {
        let ui = self.ui();
        if ui.get_confirm_open() {
            ui.set_confirm_open(false);
            *self.pending.borrow_mut() = None;
            return;
        }
        if ui.get_detail_open() || ui.get_palette_open() || ui.get_search_open() || ui.get_notif_open() || ui.get_day_open() {
            ui.invoke_close_overlays();
            return;
        }
        let prev = self.nav.borrow_mut().back(self.route(), |r| self.route_exists(r));
        if let Some(prev) = prev {
            self.show_route(prev);
        }
    }

    fn nav_forward(&self) {
        let next = self.nav.borrow_mut().forward(self.route(), |r| self.route_exists(r));
        if let Some(next) = next {
            self.show_route(next);
        }
    }

    /// 詳細パネルで入力中の内容を保存する。値が変わっていないものは書かない
    /// （update_task は題名やメモの変更でも繰り返しの将来分を作り直すため）
    fn commit_drafts(&self) {
        let d = std::mem::take(&mut *self.drafts.borrow_mut());
        let current = |id: &str| self.state.data.lock().tasks.iter().find(|t| t.id == id).cloned();
        if let Some((id, title)) = d.title {
            if let Some(t) = current(&id) {
                if !title.trim().is_empty() && t.title != title {
                    self.patch(&id, TaskPatch { title: Some(title), ..Default::default() });
                }
            }
        }
        if let Some((id, notes)) = d.notes {
            if current(&id).is_some_and(|t| t.notes != notes) {
                self.patch(&id, TaskPatch { notes: Some(notes), ..Default::default() });
            }
        }
        if let Some((tid, sid, title)) = d.subtask {
            let old = current(&tid).and_then(|t| t.subtasks.into_iter().find(|s| s.id == sid));
            if old.is_some_and(|s| s.title != title) && !title.trim().is_empty() {
                self.mutate(
                    |d| format!("{}の名前の変更", subtask_label(d, &tid, &sid)),
                    |s| subtasks::rename_subtask(s, &tid, &sid, &title),
                );
            }
        }
    }

    /// 削除して「元に戻す」付きのトーストを出す
    fn delete_with_undo(&self, id: &str) {
        let Some(name) = self.state.data.lock().tasks.iter().find(|t| t.id == id).map(|t| quoted(&t.title)) else {
            return;
        };
        if self.detail_id.borrow().as_deref() == Some(id) {
            *self.detail_id.borrow_mut() = None;
        }
        if self.mutate(|_| format!("{name}の削除"), |s| tasks::delete_task(s, id)).is_some() {
            self.toast(format!("{name}を削除しました"), true, false);
        }
    }

    /// 完了・未完了を切り替える。完了にしたときは「元に戻す」付きのトーストを出す
    /// （今日ビューでは期限切れのタスクを完了にすると一覧から消えるため）
    fn toggle_with_undo(&self, id: &str) {
        let label = |d: &AppData| {
            let done = d.tasks.iter().any(|t| t.id == id && t.status == TaskStatus::Done);
            format!("{}の{}", task_label(d, id), if done { "完了解除" } else { "完了" })
        };
        if let Some(t) = self.mutate(label, |s| tasks::toggle_task(s, id)) {
            if t.status == TaskStatus::Done {
                self.toast(format!("{}を完了にしました", quoted(&t.title)), true, false);
            }
        }
    }

    /// マウスが乗っているタスク、なければ開いている詳細のタスクをコピーする
    fn copy_task(&self) {
        let ui = self.ui();
        let hover = ui.global::<Bridge>().get_hover_task().to_string();
        let detail = if ui.get_detail_open() { self.detail_id.borrow().clone() } else { None };
        let found = {
            let data = self.state.data.lock();
            [Some(hover), detail].into_iter().flatten()
                .find_map(|id| data.tasks.iter().find(|t| t.id == id).cloned())
        };
        let Some(task) = found else { return };
        let text = match arboard::Clipboard::new().and_then(|mut c| c.set_text(task.title.clone())) {
            Ok(()) => Some(task.title.clone()),
            Err(e) => {
                eprintln!("clipboard: {e}");
                None
            }
        };
        self.toast(format!("{}をコピーしました", quoted(&task.title)), false, false);
        *self.clip.borrow_mut() = Some(Clip { task, text });
    }

    fn clipboard_has_task(&self) -> bool {
        match self.clip.borrow().as_ref() {
            None => false,
            Some(Clip { text: None, .. }) => true,
            Some(Clip { text: Some(text), .. }) => {
                arboard::Clipboard::new().and_then(|mut c| c.get_text()).is_ok_and(|t| &t == text)
            }
        }
    }

    /// コピーしたタスクを今の画面に貼り付ける。今日なら期日を今日に、受信箱ならプロジェクトなしに、
    /// プロジェクトの画面ならそのプロジェクトにする。ほかの画面ではコピー元と同じ
    fn paste_task(&self) {
        if !self.clipboard_has_task() {
            return;
        }
        let Some(mut task) = self.clip.borrow().as_ref().map(|c| c.task.clone()) else { return };
        let (view, pid) = self.route();
        match view.as_str() {
            "today" => task.due = Some(self.today.get().format("%Y-%m-%d").to_string()),
            "inbox" => task.project_id = None,
            "project" => task.project_id = Some(pid.to_string()),
            _ => {}
        }
        let name = quoted(&task.title);
        if self.mutate(|_| format!("{name}の貼り付け"), |s| tasks::paste_task(s, &task)).is_some() {
            self.toast(format!("{name}を貼り付けました"), true, false);
        }
    }

    fn patch(&self, id: &str, patch: TaskPatch) {
        let label = patch_label(&self.state.data.lock(), id, &patch);
        self.mutate(|_| label, |s| tasks::update_task(s, id, patch));
    }

    /// ファイルを選んで読み込む。読み込む前にバックアップを 1 つ残し、読み込んだ後も「元に戻す」で戻せる
    fn import_with_dialog(&self, start_dir: Option<&Path>) {
        let Some(path) = io::pick_import_file(start_dir) else { return };
        let name = quoted(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        if let Err(e) = store::backup_before(self.state.dir(), "import", Local::now()) {
            eprintln!("backup failed: {e}");
        }
        *self.drafts.borrow_mut() = Drafts::default();
        if self.mutate(|_| format!("{name}のインポート"), |s| io::import_from(s, &path)).is_some() {
            *self.detail_id.borrow_mut() = None;
            self.refresh();
            self.toast(format!("{name}を読み込みました"), true, false);
        }
    }

    fn palette_items(&self, q: &str) -> Vec<Command> {
        let cmd = |label: &str, sub: &str, view: &str, pid: &str| Command {
            label: label.into(),
            sub: sub.into(),
            view: view.into(),
            project_id: pid.into(),
        };
        let mut items = vec![
            cmd("→ 今日", "T", "today", ""),
            cmd("→ 受信箱", "I", "inbox", ""),
            cmd("→ 今後", "U", "upcoming", ""),
            cmd("→ プロジェクト一覧", "P", "projects", ""),
            cmd("→ タグ", "#", "tags", ""),
            cmd("→ 設定", ",", "settings", ""),
        ];
        for p in &self.state.data.lock().projects {
            items.push(cmd(&format!("▸ {}", p.name), "PROJECT", "project", &p.id));
        }
        let q = q.to_lowercase();
        items.retain(|it| q.is_empty() || it.label.to_lowercase().contains(&q));
        items
    }
}

impl App {
    /// 確認ダイアログを出す。答えは on_confirm_result で受ける
    fn ask(&self, pending: Pending, message: String, ok_label: &str) {
        let ui = self.ui();
        ui.set_confirm_message(message.into());
        ui.set_confirm_ok_label(ok_label.into());
        *self.pending.borrow_mut() = Some(pending);
        ui.set_confirm_open(true);
    }

    fn run_pending(&self, pending: Pending) {
        match pending {
            Pending::ResetData => {
                if let Err(e) = store::backup_before(self.state.dir(), "reset", Local::now()) {
                    eprintln!("backup failed: {e}");
                }
                *self.detail_id.borrow_mut() = None;
                *self.drafts.borrow_mut() = Drafts::default();
                if self.mutate(|_| "サンプルデータへのリセット".into(), io::reset_sample_data).is_some() {
                    self.toast("サンプルデータに戻しました".into(), true, false);
                }
            }
        }
    }
}

/// トーストに入れる名前。長いと「を取り消しました」が見切れるので、名前だけを 20 字に縮める
fn quoted(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= 20 {
        format!("「{name}」")
    } else {
        format!("「{}…」", chars[..19].iter().collect::<String>())
    }
}

fn project_label(d: &AppData, id: &str) -> String {
    let name = d.projects.iter().find(|p| p.id == id).map(|p| quoted(&p.name)).unwrap_or_default();
    format!("プロジェクト{name}")
}

fn task_label(d: &AppData, id: &str) -> String {
    d.tasks.iter().find(|t| t.id == id).map(|t| quoted(&t.title)).unwrap_or_else(|| "タスク".into())
}

fn subtask_label(d: &AppData, task_id: &str, subtask_id: &str) -> String {
    let name = d.tasks.iter()
        .find(|t| t.id == task_id)
        .and_then(|t| t.subtasks.iter().find(|s| s.id == subtask_id))
        .map(|s| quoted(&s.title))
        .unwrap_or_default();
    format!("サブタスク{name}")
}

/// 「『abc』の期日の変更」など。プロジェクトの変更は「移動」と呼ぶ
fn patch_label(d: &AppData, id: &str, p: &TaskPatch) -> String {
    let name = task_label(d, id);
    if p.project_id.is_some() {
        return format!("{name}の移動");
    }
    let what = if p.title.is_some() {
        "タイトル"
    } else if p.notes.is_some() {
        "メモ"
    } else if p.due.is_some() {
        "期日"
    } else if p.priority.is_some() {
        "優先度"
    } else if p.tag_ids.is_some() {
        "タグ"
    } else if p.interval.is_some() || p.recurrence_end.is_some() {
        "繰り返し"
    } else if p.status.is_some() {
        "状態"
    } else {
        "内容"
    };
    format!("{name}の{what}の変更")
}

fn set_bridge_today(ui: &AppWindow, t: NaiveDate) {
    let bridge = ui.global::<Bridge>();
    bridge.set_today_year(t.year());
    bridge.set_today_month(t.month() as i32);
    bridge.set_today_day(t.day() as i32);
}

/// 長いパスなどを「先頭…末尾」に縮める
fn shorten_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let head = max * 2 / 5;
    let tail = max - head - 1;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

#[cfg(test)]
mod tests {
    use crate::models;

    #[test]
    fn labels_shorten_only_the_name() {
        assert_eq!(super::quoted("abc"), "「abc」");
        let long = super::quoted("とても長いタスクの名前がここに入っていて最後まで読めない");
        assert_eq!(long.chars().count(), 22);
        assert!(long.ends_with("…」"));
        let mut d = crate::seed::sample_data();
        d.tasks[0].title = "abc".into();
        let id = d.tasks[0].id.clone();
        let p = models::TaskPatch { due: Some(None), ..Default::default() };
        assert_eq!(super::patch_label(&d, &id, &p), "「abc」の期日の変更");
        let p = models::TaskPatch { project_id: Some(None), ..Default::default() };
        assert_eq!(super::patch_label(&d, &id, &p), "「abc」の移動");
    }

    #[test]
    fn shorten_middle_keeps_both_ends() {
        assert_eq!(super::shorten_middle("abc", 10), "abc");
        let s = super::shorten_middle("保存しました: C:\\Users\\naoki\\Documents\\very\\long\\folder\\qota-export.json", 30);
        assert_eq!(s.chars().count(), 30);
        assert!(s.starts_with("保存しました"));
        assert!(s.ends_with("qota-export.json"));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // レンダラは Cargo.toml でソフトウェアレンダラだけに絞っている（OpenGL だと常駐メモリが約 60MB 増える）
    #[cfg(windows)]
    window_state::make_dpi_aware();

    let dir = store::resolve_data_dir();
    std::fs::create_dir_all(&dir)?;
    // 二重起動の防止。データに触る前（起動時の整理より前）にロックを取り、main の最後まで持つ
    let Some(_instance_lock) = instance::lock_or_activate(&dir)? else {
        return Ok(());
    };
    let state = AppState::load_or_init(dir.clone())?;
    let ui = AppWindow::new()?;

    // 起動時の大きさを画面の作業領域に収め、前回の大きさと最大化を戻す
    let saved_window = window_state::load(&dir);
    #[cfg(windows)]
    if let Some(area) = window_state::work_area() {
        let (w, h, x, y) = window_state::initial_geometry(saved_window, area);
        ui.window().set_size(slint::PhysicalSize::new(w, h));
        ui.window().set_position(slint::PhysicalPosition::new(x, y));
    }
    if saved_window.is_some_and(|s| s.maximized) {
        ui.window().set_maximized(true);
    }

    let app = Rc::new(App {
        state,
        ui: ui.as_weak(),
        expanded: RefCell::default(),
        detail_id: RefCell::default(),
        active: Rc::new(VecModel::default()),
        done: Rc::new(VecModel::default()),
        projects: Rc::new(VecModel::default()),
        detail_subtasks: Rc::new(VecModel::default()),
        detail_tags: Rc::new(VecModel::default()),
        commands: Rc::new(VecModel::default()),
        columns: Rc::new(VecModel::default()),
        cal_cells: Rc::new(VecModel::default()),
        day_tasks: Rc::new(VecModel::default()),
        search_results: Rc::new(VecModel::default()),
        notif_overdue: Rc::new(VecModel::default()),
        notif_today: Rc::new(VecModel::default()),
        cal_month: Cell::new({
            let t = view::today();
            (t.year(), t.month() as i32)
        }),
        open_day: Cell::new(None),
        search_query: RefCell::default(),
        toast_timer: Timer::default(),
        pending: RefCell::default(),
        undo_stack: RefCell::default(),
        nav: RefCell::default(),
        drafts: RefCell::default(),
        today: Cell::new(view::today()),
        day_timer: Timer::default(),
        clip: RefCell::new(None),
    });

    // サイドバーと「完了済み」の開閉、今日の並び順を戻す。開発用フックで起動したときはクリック位置の前提が崩れるので戻さない
    let dev_hooks = std::env::var_os("QOTA_STEPS").is_some() || std::env::var_os("QOTA_OPEN").is_some();
    if !dev_hooks {
        let prefs = window_state::load_prefs(&dir);
        ui.set_sidebar_open(prefs.sidebar_open);
        ui.set_archive_open(prefs.archive_open);
        ui.set_today_sort(view::SORT_KEYS.iter().position(|k| *k == prefs.today_sort).unwrap_or(0) as i32);
    }

    ui.set_active_tasks(app.active.clone().into());
    ui.set_done_tasks(app.done.clone().into());
    ui.set_projects(app.projects.clone().into());
    ui.set_detail_subtasks(app.detail_subtasks.clone().into());
    ui.set_detail_tags(app.detail_tags.clone().into());
    ui.set_commands(app.commands.clone().into());
    ui.set_columns(app.columns.clone().into());
    ui.set_cal_cells(app.cal_cells.clone().into());
    ui.set_day_tasks(app.day_tasks.clone().into());
    ui.set_search_results(app.search_results.clone().into());
    ui.set_notif_overdue(app.notif_overdue.clone().into());
    ui.set_notif_today(app.notif_today.clone().into());
    ui.set_data_path(app.state.path.to_string_lossy().as_ref().into());

    let bridge = ui.global::<Bridge>();
    set_bridge_today(&ui, app.today.get());
    {
        let w = Rc::downgrade(&app);
        app.day_timer.start(slint::TimerMode::Repeated, Duration::from_secs(60), move || {
            if let Some(a) = w.upgrade() {
                a.check_date();
            }
        });
    }
    bridge.on_payload(|id| slint::DataTransfer::from(id));
    bridge.on_payload_text(|d| d.plain_text().unwrap_or_default());
    bridge.on_month_days(|y, m, monday_first| {
        ModelRc::new(VecModel::from(view::month_days(y, m, monday_first)))
    });
    {
        let w = Rc::downgrade(&app);
        bridge.on_undo(move || if let Some(a) = w.upgrade() { a.step_history(true) });
        let w = Rc::downgrade(&app);
        bridge.on_redo(move || if let Some(a) = w.upgrade() { a.step_history(false) });
        let w = Rc::downgrade(&app);
        bridge.on_nav_back(move || if let Some(a) = w.upgrade() { a.nav_back() });
        let w = Rc::downgrade(&app);
        bridge.on_nav_forward(move || if let Some(a) = w.upgrade() { a.nav_forward() });
        let w = Rc::downgrade(&app);
        bridge.on_copy_task(move || if let Some(a) = w.upgrade() { a.copy_task() });
        let w = Rc::downgrade(&app);
        bridge.on_paste_task(move || if let Some(a) = w.upgrade() { a.paste_task() });
        let w = Rc::downgrade(&app);
        bridge.on_clipboard_has_task(move || w.upgrade().is_some_and(|a| a.clipboard_has_task()));
    }

    // マウスの戻る・進むボタン。Slint では押した位置の部品にクリックとして届くので、
    // winit のイベントを部品より先に受け取って画面全体で拾う
    {
        use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
        let w = Rc::downgrade(&app);
        ui.window().on_winit_window_event(move |_, event| {
            let winit::event::WindowEvent::MouseInput { state: winit::event::ElementState::Pressed, button, .. } = event else {
                return EventResult::Propagate;
            };
            let back = match button {
                winit::event::MouseButton::Back => true,
                winit::event::MouseButton::Forward => false,
                _ => return EventResult::Propagate,
            };
            // イベント処理の途中で画面を組み替えないよう、次のループで実行する
            let w = w.clone();
            Timer::single_shot(Duration::ZERO, move || {
                if let Some(a) = w.upgrade() {
                    if back { a.nav_back() } else { a.nav_forward() }
                }
            });
            EventResult::PreventDefault
        });
    }

    // コールバックの登録を短く書くためのマクロ。`app` を弱参照で捕まえる
    macro_rules! on {
        ($setter:ident, |$a:ident $(, $arg:ident)*| $body:expr) => {{
            let weak = Rc::downgrade(&app);
            ui.$setter(move |$($arg),*| {
                if let Some($a) = weak.upgrade() {
                    $body
                }
            });
        }};
    }

    // ───── routing ─────
    on!(on_navigate, |a, v, p| a.go(v, p));

    // ───── tasks ─────
    on!(on_add_task, |a, title| {
        let ui = a.ui();
        let project_id = (ui.get_view() == "project").then(|| ui.get_project_id().to_string());
        // 今日ビューで追加したものは期日を今日にする（期日なしだと今日ビューに出ず、受信箱へ行ってしまう）
        let due = (ui.get_view() == "today").then(|| view::today().format("%Y-%m-%d").to_string());
        if !title.trim().is_empty() {
            a.mutate(
                |_| format!("{}の追加", quoted(title.trim())),
                |s| tasks::create_task(s, NewTask { title: title.to_string(), project_id, due, ..Default::default() }),
            );
        }
    });
    on!(on_sort_changed, |a| a.refresh());
    on!(on_open_task, |a, id| {
        *a.detail_id.borrow_mut() = Some(id.to_string());
        a.refresh();
        a.ui().set_detail_open(true);
    });
    on!(on_toggle_task, |a, id| a.toggle_with_undo(&id));
    on!(on_toggle_status, |a, id| a.toggle_with_undo(&id));
    on!(on_toggle_expanded, |a, id| {
        let id = id.to_string();
        {
            let mut ex = a.expanded.borrow_mut();
            if !ex.remove(&id) {
                ex.insert(id);
            }
        }
        a.refresh();
    });
    on!(on_set_priority, |a, id, p| a.patch(&id, TaskPatch { priority: Some(view::int_to_priority(p)), ..Default::default() }));
    on!(on_set_due, |a, id, y, m, d| a.patch(&id, TaskPatch { due: Some(view::iso(y, m, d)), ..Default::default() }));
    on!(on_clear_due, |a, id| a.patch(&id, TaskPatch { due: Some(None), ..Default::default() }));
    on!(on_update_title, |a, id, title| {
        a.drafts.borrow_mut().title = None;
        if !title.trim().is_empty() {
            a.patch(&id, TaskPatch { title: Some(title.to_string()), ..Default::default() });
        }
    });
    on!(on_update_notes, |a, id, notes| {
        a.drafts.borrow_mut().notes = None;
        a.patch(&id, TaskPatch { notes: Some(notes.to_string()), ..Default::default() });
    });
    on!(on_draft_title, |a, id, title| a.drafts.borrow_mut().title = Some((id.to_string(), title.to_string())));
    on!(on_draft_notes, |a, id, notes| a.drafts.borrow_mut().notes = Some((id.to_string(), notes.to_string())));
    on!(on_draft_subtask, |a, tid, sid, title| {
        a.drafts.borrow_mut().subtask = Some((tid.to_string(), sid.to_string(), title.to_string()));
    });
    on!(on_commit_drafts, |a| a.commit_drafts());
    on!(on_set_project, |a, id, index| {
        let pid = (index > 0)
            .then(|| a.state.data.lock().projects.get(index as usize - 1).map(|p| p.id.clone()))
            .flatten();
        let (name, place) = {
            let d = a.state.data.lock();
            let place = pid.as_ref()
                .and_then(|p| d.projects.iter().find(|x| &x.id == p))
                .map(|p| quoted(&p.name))
                .unwrap_or_else(|| "受信箱".into());
            (task_label(&d, &id), place)
        };
        let moved = a.state.data.lock().tasks.iter().any(|t| t.id == id.as_str() && t.project_id != pid);
        a.patch(&id, TaskPatch { project_id: Some(pid), ..Default::default() });
        // 今の一覧から消えることがあるので、移した先を知らせて「元に戻す」を付ける
        if moved {
            a.toast(format!("{name}を{place}に移動しました"), true, false);
        }
    });
    on!(on_set_repeating, |a, id, on| {
        let interval = on.then_some(Interval::Daily);
        a.patch(&id, TaskPatch { interval: Some(interval), ..Default::default() });
    });
    on!(on_set_interval, |a, id, index| {
        let interval = view::INTERVALS.get(index as usize).copied();
        a.patch(&id, TaskPatch { interval: Some(interval), ..Default::default() });
    });
    on!(on_set_recurrence_end, |a, id, y, m, d| a.patch(&id, TaskPatch { recurrence_end: Some(view::iso(y, m, d)), ..Default::default() }));
    on!(on_clear_recurrence_end, |a, id| a.patch(&id, TaskPatch { recurrence_end: Some(None), ..Default::default() }));
    on!(on_toggle_tag, |a, id, tag_id| {
        let current = a.state.data.lock().tasks.iter()
            .find(|t| t.id == id.as_str())
            .map(|t| t.tag_ids.clone())
            .unwrap_or_default();
        let next = if current.iter().any(|x| x == tag_id.as_str()) {
            current.into_iter().filter(|x| x != tag_id.as_str()).collect()
        } else {
            let mut v = current;
            v.push(tag_id.to_string());
            v
        };
        a.patch(&id, TaskPatch { tag_ids: Some(next), ..Default::default() });
    });
    on!(on_duplicate_task, |a, id| {
        a.mutate(|d| format!("{}の複製", task_label(d, &id)), |s| tasks::duplicate_task(s, &id));
    });
    on!(on_delete_task, |a, id| a.delete_with_undo(&id));
    on!(on_toast_action_clicked, |a| a.step_history(true));
    on!(on_confirm_result, |a, ok| {
        let pending = a.pending.borrow_mut().take();
        if let (true, Some(p)) = (ok, pending) {
            a.run_pending(p);
        }
    });

    // ───── subtasks ─────
    on!(on_toggle_subtask, |a, tid, sid| {
        let label = |d: &AppData| {
            let done = d.tasks.iter().find(|t| t.id == tid.as_str())
                .and_then(|t| t.subtasks.iter().find(|s| s.id == sid.as_str()))
                .is_some_and(|s| s.done);
            format!("{}の{}", subtask_label(d, &tid, &sid), if done { "完了解除" } else { "完了" })
        };
        a.mutate(label, |s| subtasks::toggle_subtask(s, &tid, &sid));
    });
    on!(on_add_subtask, |a, tid, title| {
        if !title.trim().is_empty() {
            a.mutate(|_| format!("サブタスク{}の追加", quoted(title.trim())), |s| subtasks::add_subtask(s, &tid, &title));
        }
    });
    on!(on_delete_subtask, |a, tid, sid| {
        let name = subtask_label(&a.state.data.lock(), &tid, &sid);
        if a.mutate(|_| format!("{name}の削除"), |s| subtasks::delete_subtask(s, &tid, &sid)).is_some() {
            a.toast(format!("{name}を削除しました"), true, false);
        }
    });
    on!(on_rename_subtask, |a, tid, sid, title| {
        a.drafts.borrow_mut().subtask = None;
        if !title.trim().is_empty() {
            a.mutate(
                |d| format!("{}の名前の変更", subtask_label(d, &tid, &sid)),
                |s| subtasks::rename_subtask(s, &tid, &sid, &title),
            );
        }
    });
    on!(on_drop_subtask, |a, tid, sid, slot| {
        if sid.starts_with("project:") {
            return;
        }
        // slot は「取り除く前」の挿入位置。from より後ろなら 1 つ詰める
        let from = a.state.data.lock().tasks.iter()
            .find(|t| t.id == tid.as_str())
            .and_then(|t| t.subtasks.iter().position(|s| s.id == sid.as_str()));
        if let (Some(from), true) = (from, slot >= 0) {
            let slot = slot as usize;
            let to = if slot > from { slot - 1 } else { slot };
            if to != from {
                a.mutate(
                    |d| format!("{}のサブタスクの並び替え", task_label(d, &tid)),
                    |s| subtasks::move_subtask_to(s, &tid, from, to),
                );
            }
        }
    });

    // ───── projects ─────
    on!(on_add_project, |a, name| {
        if !name.trim().is_empty() {
            a.mutate(
                |_| format!("プロジェクト{}の追加", quoted(name.trim())),
                |s| projects::create_project(s, NewProject { name: name.to_string(), color: None }),
            );
        }
    });

    on!(on_add_project_color, |a, name, color| {
        if !name.trim().is_empty() {
            a.mutate(
                |_| format!("プロジェクト{}の追加", quoted(name.trim())),
                |s| projects::create_project(s, NewProject { name: name.trim().to_string(), color: Some(color.to_string()) }),
            );
        }
    });
    on!(on_update_project, |a, id, name, color| {
        let patch = ProjectPatch {
            name: (!name.trim().is_empty()).then(|| name.trim().to_string()),
            color: (!color.is_empty()).then(|| color.to_string()),
            archived: None,
        };
        a.mutate(|d| format!("{}の変更", project_label(d, &id)), |s| projects::update_project(s, &id, patch));
    });
    on!(on_delete_project, |a, id| {
        let name = project_label(&a.state.data.lock(), &id);
        if a.mutate(|_| format!("{name}の削除"), |s| projects::delete_project(s, &id)).is_some() {
            a.toast(format!("{name}を削除しました"), true, false);
        }
    });
    on!(on_move_project, |a, payload, before| {
        // サイドバーとプロジェクト一覧の両方から来る。before はその前に入れるプロジェクト（空なら末尾）
        let Some(id) = payload.strip_prefix("project:").map(str::to_string) else { return };
        let before = (!before.is_empty()).then(|| before.to_string());
        a.mutate(|_| "プロジェクトの並び替え".into(), |s| projects::move_project_before(s, &id, before.as_deref()));
    });
    on!(on_add_tag, |a, name| {
        // IME だと全角の ＃ が入るので、どちらも外す
        let name = name.trim().trim_start_matches(['#', '＃']).trim().to_string();
        if name.is_empty() {
            return;
        }
        if a.state.data.lock().tags.iter().any(|t| t.name.eq_ignore_ascii_case(&name)) {
            a.toast(format!("タグ「#{name}」はすでにあります"), false, true);
            return;
        }
        a.mutate(|_| format!("タグ{}の追加", quoted(&format!("#{name}"))), |s| tags::create_tag(s, &name));
    });
    on!(on_delete_tag, |a, id| {
        let name = a.state.data.lock().tags.iter().find(|t| t.id == id.as_str())
            .map(|t| quoted(&format!("#{}", t.name)))
            .unwrap_or_default();
        if a.mutate(|_| format!("タグ{name}の削除"), |s| tags::delete_tag(s, &id)).is_some() {
            a.toast(format!("タグ{name}を削除しました"), true, false);
        }
    });

    // ───── calendar ─────
    on!(on_cal_shift, |a, delta| {
        let (y, m) = a.cal_month.get();
        let total = y * 12 + (m - 1) + delta;
        a.cal_month.set((total.div_euclid(12), total.rem_euclid(12) + 1));
        a.refresh();
    });
    on!(on_cal_today, |a| {
        let t = a.today.get();
        a.cal_month.set((t.year(), t.month() as i32));
        a.refresh();
    });
    on!(on_open_day, |a, y, m, d| {
        a.open_day.set(Some((y, m, d)));
        a.refresh();
        a.ui().set_day_open(true);
    });

    // ───── search ─────
    on!(on_search_query, |a, q| {
        *a.search_query.borrow_mut() = q.to_string();
        a.run_search(&q);
    });

    // ───── palette ─────
    on!(on_palette_query, |a, q| view::sync(&a.commands, a.palette_items(&q)));
    on!(on_palette_run, |a, i| {
        use slint::Model;
        if let Some(c) = a.commands.row_data(i as usize) {
            a.go(c.view, c.project_id);
        }
    });

    // ───── settings ─────
    on!(on_set_dark, |a, dark| {
        let mode = if dark { Mode::Dark } else { Mode::Light };
        a.apply(settings::update_settings(&a.state, SettingsPatch { mode: Some(mode), ..Default::default() }));
    });
    on!(on_export_json, |a| match io::export_json(&a.state) {
        Ok(path) => a.toast(format!("保存しました: {path}"), false, false),
        Err(Error::Cancelled) => {}
        Err(e) => a.toast(format!("エクスポートに失敗しました: {e}"), false, true),
    });
    on!(on_import_json, |a| a.import_with_dialog(None));
    on!(on_restore_backup, |a| {
        let dir = store::backups_dir(a.state.dir());
        a.import_with_dialog(Some(&dir));
    });
    on!(on_open_backups, |a| {
        let dir = store::backups_dir(a.state.dir());
        let _ = std::fs::create_dir_all(&dir);
        // explorer は成功しても終了コード 1 を返すので、結果は見ない
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    });
    on!(on_reset_data, |a| {
        a.ask(Pending::ResetData, "サンプルデータに戻します。よろしいですか？".into(), "リセット");
    });


    app.refresh();

    // 開発用: スクリーンショットで確認するため、起動直後に特定の画面を開く
    match std::env::var("QOTA_OPEN").as_deref() {
        Ok("detail") => {
            use slint::Model;
            if let Some(t) = app.active.row_data(0) {
                ui.invoke_open_task(t.id);
            }
        }
        Ok("palette") => {
            ui.invoke_palette_query(SharedString::new());
            ui.set_palette_open(true);
        }
        Ok(v @ ("settings" | "upcoming" | "calendar" | "projects" | "tags" | "inbox")) => {
            ui.invoke_navigate(v.into(), SharedString::new())
        }
        Ok("search") => {
            ui.invoke_search_query(SharedString::new());
            ui.set_search_open(true);
        }
        Ok("notif") => ui.set_notif_open(true),
        Ok("confirm") => ui.invoke_reset_data(),
        Ok("toast") => app.toast("「サンプル」を削除しました".into(), true, false),
        _ => {}
    }
    // 開発用: QOTA_STEPS="click:x,y;type:abc;esc;enter;close" を、起動 5 秒後から 1 秒おきに実行する。
    // クリック位置は論理座標。実際のマウスやキーボードは使わず、ウィンドウにイベントを直接送る（動作確認用）
    let steps: Vec<String> = std::env::var("QOTA_STEPS").unwrap_or_default()
        .split(';')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    for (i, step) in steps.into_iter().enumerate() {
        let weak = ui.as_weak();
        Timer::single_shot(Duration::from_millis(5000 + 1000 * i as u64), move || {
            use slint::platform::{Key, PointerEventButton, WindowEvent};
            let Some(ui) = weak.upgrade() else { return };
            let w = ui.window();
            let press = |text: SharedString| {
                w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                w.dispatch_event(WindowEvent::KeyReleased { text });
            };
            if let Some(xy) = step.strip_prefix("click:") {
                let Some((x, y)) = xy.split_once(',') else { return };
                let (Ok(x), Ok(y)) = (x.trim().parse::<f32>(), y.trim().parse::<f32>()) else { return };
                let position = slint::LogicalPosition::new(x, y);
                w.dispatch_event(WindowEvent::PointerMoved { position });
                w.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
                w.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
            } else if let Some((release, args)) = step.strip_prefix("drag:").map(|a| (true, a))
                .or_else(|| step.strip_prefix("hold:").map(|a| (false, a)))
            {
                // drag:x1,y1,x2,y2 押して、少しずつ動かして、離す。
                // hold: は離さない（ドラッグ中の表示を撮る用。release:x,y で離す）
                let v: Vec<f32> = args.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                let [x1, y1, x2, y2] = v[..] else { return };
                let at = |t: f32| slint::LogicalPosition::new(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t);
                w.dispatch_event(WindowEvent::PointerMoved { position: at(0.0) });
                w.dispatch_event(WindowEvent::PointerPressed { position: at(0.0), button: PointerEventButton::Left });
                for i in 1..=10 {
                    w.dispatch_event(WindowEvent::PointerMoved { position: at(i as f32 / 10.0) });
                }
                if release {
                    w.dispatch_event(WindowEvent::PointerReleased { position: at(1.0), button: PointerEventButton::Left });
                }
            } else if let Some(args) = step.strip_prefix("move:") {
                // move:x,y マウスを動かすだけ（ホバーの確認用）
                let v: Vec<f32> = args.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                let [x, y] = v[..] else { return };
                w.dispatch_event(WindowEvent::PointerMoved { position: slint::LogicalPosition::new(x, y) });
            } else if let Some(args) = step.strip_prefix("release:") {
                let v: Vec<f32> = args.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                let [x, y] = v[..] else { return };
                let position = slint::LogicalPosition::new(x, y);
                w.dispatch_event(WindowEvent::PointerMoved { position });
                w.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
            } else if let Some((shift, args)) = step.strip_prefix("wheel:").map(|a| (false, a))
                .or_else(|| step.strip_prefix("shiftwheel:").map(|a| (true, a)))
            {
                // wheel:x,y,dy / shiftwheel:x,y,dy  ホイールを回す（dy が負なら下へ回したのと同じ）
                let v: Vec<f32> = args.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                let [x, y, dy] = v[..] else { return };
                let position = slint::LogicalPosition::new(x, y);
                if shift {
                    w.dispatch_event(WindowEvent::KeyPressed { text: Key::Shift.into() });
                }
                w.dispatch_event(WindowEvent::PointerMoved { position });
                w.dispatch_event(WindowEvent::PointerScrolled { position, delta_x: 0.0, delta_y: dy });
                if shift {
                    w.dispatch_event(WindowEvent::KeyReleased { text: Key::Shift.into() });
                }
            } else if let Some(text) = step.strip_prefix("type:") {
                for c in text.chars() {
                    press(c.to_string().into());
                }
            } else if let Some((modifier, key)) = step.split_once(':').filter(|(m, _)| *m == "ctrl" || *m == "alt") {
                // ctrl:z / ctrl:y / alt:left / alt:right
                let modifier: SharedString = if modifier == "ctrl" { Key::Control.into() } else { Key::Alt.into() };
                let key: SharedString = match key {
                    "left" => Key::LeftArrow.into(),
                    "right" => Key::RightArrow.into(),
                    k => k.into(),
                };
                w.dispatch_event(WindowEvent::KeyPressed { text: modifier.clone() });
                press(key);
                w.dispatch_event(WindowEvent::KeyReleased { text: modifier });
            } else if step == "esc" {
                press(Key::Escape.into());
            } else if step == "enter" {
                press(Key::Return.into());
            } else if step == "close" {
                w.dispatch_event(WindowEvent::CloseRequested);
            }
        });
    }

    // ウィンドウの × で閉じるとき: 入力中の内容を保存し、大きさと最大化を window.json に残す
    {
        let weak = Rc::downgrade(&app);
        let dir = dir.clone();
        ui.window().on_close_requested(move || {
            if let Some(a) = weak.upgrade() {
                a.commit_drafts();
                let ui = a.ui();
                let prefs = window_state::UiPrefs {
                    sidebar_open: ui.get_sidebar_open(),
                    archive_open: ui.get_archive_open(),
                    today_sort: view::SORT_KEYS.get(ui.get_today_sort() as usize).copied().unwrap_or_default(),
                };
                if let Err(e) = window_state::save_prefs(&dir, &prefs) {
                    eprintln!("failed to save ui-state.json: {e}");
                }
                let w = ui.window();
                let maximized = w.is_maximized();
                // 最大化・最小化中は通常時の大きさが取れないので、前回の値を引き継ぐ
                let size = if maximized || w.is_minimized() {
                    window_state::load(&dir).map(|p| (p.width, p.height))
                } else {
                    Some((w.size().width, w.size().height))
                };
                // 最小サイズを下回る大きさに縮めた直後などに異常な値が返ることがあるので、その場合は保存しない
                if let Some((width, height)) = size.filter(|&(w, h)| (1..=16384).contains(&w) && (1..=16384).contains(&h)) {
                    let _ = window_state::save(&dir, &window_state::WindowState { width, height, maximized });
                }
            }
            slint::CloseRequestResponse::HideWindow
        });
    }

    ui.run()?;
    Ok(())
}
