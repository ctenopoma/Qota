// 操作の取り消し・やり直し (Ctrl+Z / Ctrl+Y) と、画面の戻る・進む（マウスの戻る・進むボタン）の履歴。

use crate::models::AppData;

const UNDO_LIMIT: usize = 100;
const NAV_LIMIT: usize = 50;

/// 操作の直前のデータを丸ごと積む。データは数 KB なので 100 件積んでも軽い。
/// 後入れ先出しで 1 つずつ戻すので、途中の操作を飛ばして戻すことは無い。
/// 説明（「『abc』の削除」など）は状態の移り変わりに付くので、取り消し・やり直しでスナップショットと一緒に移す
#[derive(Default)]
pub struct UndoStack {
    undo: Vec<(AppData, String)>,
    redo: Vec<(AppData, String)>,
}

impl UndoStack {
    /// 操作の前に呼ぶ。新しい操作をしたら、やり直しの履歴は捨てる
    pub fn record(&mut self, before: AppData, label: String) {
        self.undo.push((before, label));
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// 戻した先のデータと、取り消した操作の説明を返す。今のデータはやり直し用に残す
    pub fn undo(&mut self, current: AppData) -> Option<(AppData, String)> {
        let (prev, label) = self.undo.pop()?;
        self.redo.push((current, label.clone()));
        Some((prev, label))
    }

    pub fn redo(&mut self, current: AppData) -> Option<(AppData, String)> {
        let (next, label) = self.redo.pop()?;
        self.undo.push((current, label.clone()));
        Some((next, label))
    }
}

/// 画面の履歴。(view, project-id) を積む
#[derive(Default)]
pub struct NavStack<R> {
    back: Vec<R>,
    forward: Vec<R>,
}

impl<R: PartialEq> NavStack<R> {
    /// 画面を移ったとき。移る前の画面を積み、進む履歴は捨てる
    pub fn visit(&mut self, from: R) {
        self.back.push(from);
        if self.back.len() > NAV_LIMIT {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// 戻る先を返す。valid が偽のもの（削除したプロジェクトなど）は飛ばす
    pub fn back(&mut self, current: R, valid: impl Fn(&R) -> bool) -> Option<R> {
        while let Some(r) = self.back.pop() {
            if valid(&r) && r != current {
                self.forward.push(current);
                return Some(r);
            }
        }
        None
    }

    pub fn forward(&mut self, current: R, valid: impl Fn(&R) -> bool) -> Option<R> {
        while let Some(r) = self.forward.pop() {
            if valid(&r) && r != current {
                self.back.push(current);
                return Some(r);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(title: &str) -> AppData {
        let mut d = crate::seed::sample_data();
        d.tasks[0].title = title.into();
        d
    }

    #[test]
    fn undo_and_redo_walk_back_and_forth_with_labels() {
        let mut h = UndoStack::default();
        h.record(data("a"), "op1".into());
        h.record(data("b"), "op2".into());
        // 今が c。戻ると b（op2 を取り消し）、もう一度で a（op1）、それ以上は戻れない
        let (b, l) = h.undo(data("c")).unwrap();
        assert_eq!((b.tasks[0].title.as_str(), l.as_str()), ("b", "op2"));
        let (a, l) = h.undo(b).unwrap();
        assert_eq!((a.tasks[0].title.as_str(), l.as_str()), ("a", "op1"));
        assert!(h.undo(a.clone()).is_none());
        // やり直すと op1 で b、op2 で c
        let (b, l) = h.redo(a).unwrap();
        assert_eq!((b.tasks[0].title.as_str(), l.as_str()), ("b", "op1"));
        let (c, l) = h.redo(b).unwrap();
        assert_eq!((c.tasks[0].title.as_str(), l.as_str()), ("c", "op2"));
        assert!(h.redo(c.clone()).is_none());
        // やり直した後にもう一度取り消すと、同じ説明が出る
        let (_, l) = h.undo(c).unwrap();
        assert_eq!(l, "op2");
    }

    #[test]
    fn new_operation_clears_redo() {
        let mut h = UndoStack::default();
        h.record(data("a"), "op1".into());
        let (a, _) = h.undo(data("b")).unwrap();
        h.record(a, "op2".into());
        assert!(h.redo(data("x")).is_none());
    }

    #[test]
    fn nav_back_forward_and_skip_invalid() {
        let mut n = NavStack::default();
        n.visit("today");
        n.visit("deleted-project");
        n.visit("inbox");
        // 今は settings。戻ると inbox、次は削除済みを飛ばして today
        let valid = |r: &&str| *r != "deleted-project";
        assert_eq!(n.back("settings", valid), Some("inbox"));
        assert_eq!(n.back("inbox", valid), Some("today"));
        assert_eq!(n.back("today", valid), None);
        assert_eq!(n.forward("today", valid), Some("inbox"));
        assert_eq!(n.forward("inbox", valid), Some("settings"));
        // 新しく移ると進む履歴は消える
        n.back("settings", valid);
        n.visit("inbox");
        assert_eq!(n.forward("tags", valid), None);
    }
}
