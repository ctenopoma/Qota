// 今日のタイムチャートの計画。棒（PlanBlock）を 15 分刻みで置き・動かし・伸ばし・外す。
// 棒どうしは重ならない（置こうとした所が埋まっていれば、空いている次の位置に詰める）。

use uuid::Uuid;
use crate::error::{Error, Result};
use crate::models::{AppData, PlanBlock, Settings};
use crate::store::AppState;

/// 刻み（分）
pub const SLOT: u16 = 15;
/// 置いたときの長さの初期値（分）
pub const DEFAULT_LEN: u16 = 30;

/// 1 日の時間軸（0 時からの分）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Day {
    pub start: u16,
    pub end: u16,
}

impl Day {
    pub fn from_settings(s: &Settings) -> Self {
        Day { start: s.day_start_hour as u16 * 60, end: s.day_end_hour as u16 * 60 }
    }
}

fn snap(min: u16) -> u16 {
    min / SLOT * SLOT
}

/// others は (開始, 長さ)。desired 以降で、長さ len がどれとも重ならない最初の位置を返す。
/// 後ろに空きが無ければ、軸の先頭から探し直す。どこにも入らなければ None
pub fn find_slot(others: &[(u16, u16)], desired: u16, len: u16, day: Day) -> Option<u16> {
    let scan = |from: u16| -> Option<u16> {
        let mut s = snap(from).max(day.start);
        while s + len <= day.end {
            let blocked = others
                .iter()
                .filter(|(bs, bl)| *bs < s + len && s < bs + bl)
                .map(|(bs, bl)| bs + bl)
                .max();
            match blocked {
                Some(end) => s = end,
                None => return Some(s),
            }
        }
        None
    };
    scan(desired).or_else(|| scan(day.start))
}

fn others(data: &AppData, date: &str, exclude: Option<&str>) -> Vec<(u16, u16)> {
    data.plans
        .iter()
        .filter(|b| b.date == date && Some(b.id.as_str()) != exclude)
        .map(|b| (b.start_min, b.duration_min))
        .collect()
}

/// 今日のタスクを棒として置く。すでに置いてあれば、その棒を動かす。desired は 0 時からの分
pub fn place_task(state: &AppState, task_id: &str, date: &str, desired: u16, len: u16) -> Result<PlanBlock> {
    let block = {
        let mut data = state.data.lock();
        let day = Day::from_settings(&data.settings);
        let title = data.tasks.iter().find(|t| t.id == task_id)
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?
            .title.clone();
        let existing = data.plans.iter().find(|b| b.date == date && b.task_id == task_id).map(|b| b.id.clone());
        let len = snap(len).max(SLOT);
        let start = find_slot(&others(&data, date, existing.as_deref()), desired, len, day)
            .ok_or(Error::NoSpace)?;
        match existing {
            Some(id) => {
                let b = data.plans.iter_mut().find(|b| b.id == id).expect("block");
                b.start_min = start;
                b.clone()
            }
            None => {
                let b = PlanBlock {
                    id: Uuid::new_v4().to_string(),
                    task_id: task_id.to_string(),
                    title,
                    date: date.to_string(),
                    start_min: start,
                    duration_min: len,
                };
                data.plans.push(b.clone());
                b
            }
        }
    };
    state.save()?;
    Ok(block)
}

/// 棒を動かす（長さは変えない）。置き先が埋まっていれば、空いている次の位置へ
pub fn move_block(state: &AppState, block_id: &str, desired: u16) -> Result<PlanBlock> {
    let block = {
        let mut data = state.data.lock();
        let day = Day::from_settings(&data.settings);
        let (date, len) = {
            let b = data.plans.iter().find(|b| b.id == block_id)
                .ok_or_else(|| Error::NotFound(format!("block {block_id}")))?;
            (b.date.clone(), b.duration_min)
        };
        let start = find_slot(&others(&data, &date, Some(block_id)), desired, len, day)
            .ok_or(Error::NoSpace)?;
        let b = data.plans.iter_mut().find(|b| b.id == block_id).expect("block");
        b.start_min = start;
        b.clone()
    };
    state.save()?;
    Ok(block)
}

/// 棒を delta_min 分ずらす（キー操作用）。ほかの棒や軸の外に当たるなら動かさない（飛び越えない）
pub fn shift_block(state: &AppState, block_id: &str, delta_min: i32) -> Result<PlanBlock> {
    let block = {
        let mut data = state.data.lock();
        let day = Day::from_settings(&data.settings);
        let (date, start, len) = {
            let b = data.plans.iter().find(|b| b.id == block_id)
                .ok_or_else(|| Error::NotFound(format!("block {block_id}")))?;
            (b.date.clone(), b.start_min, b.duration_min)
        };
        let target = start as i32 + delta_min;
        let fits = target >= day.start as i32
            && target + len as i32 <= day.end as i32
            && !others(&data, &date, Some(block_id))
                .iter()
                .any(|(bs, bl)| (*bs as i32) < target + len as i32 && target < (*bs + *bl) as i32);
        let b = data.plans.iter_mut().find(|b| b.id == block_id).expect("block");
        if fits {
            b.start_min = target as u16;
        }
        b.clone()
    };
    state.save()?;
    Ok(block)
}

/// 棒の長さを変える（15 分以上）。次の棒や軸の終わりを越える分は切る
pub fn resize_block(state: &AppState, block_id: &str, new_len: u16) -> Result<PlanBlock> {
    let block = {
        let mut data = state.data.lock();
        let day = Day::from_settings(&data.settings);
        let (date, start) = {
            let b = data.plans.iter().find(|b| b.id == block_id)
                .ok_or_else(|| Error::NotFound(format!("block {block_id}")))?;
            (b.date.clone(), b.start_min)
        };
        let next_start = others(&data, &date, Some(block_id))
            .into_iter()
            .map(|(s, _)| s)
            .filter(|s| *s > start)
            .min()
            .unwrap_or(day.end);
        let max_len = next_start.min(day.end).saturating_sub(start).max(SLOT);
        let len = snap(new_len).max(SLOT).min(max_len);
        let b = data.plans.iter_mut().find(|b| b.id == block_id).expect("block");
        b.duration_min = len;
        b.clone()
    };
    state.save()?;
    Ok(block)
}

/// 棒を外す（タスクは消さない）
pub fn remove_block(state: &AppState, block_id: &str) -> Result<()> {
    {
        let mut data = state.data.lock();
        let before = data.plans.len();
        data.plans.retain(|b| b.id != block_id);
        if data.plans.len() == before {
            return Err(Error::NotFound(format!("block {block_id}")));
        }
    }
    state.save()
}

/// まだ置いていないタスクを、from（0 時からの分）から順に並べる。置けた数を返す
pub fn arrange_from(state: &AppState, date: &str, from: u16, task_ids: &[String]) -> Result<usize> {
    let mut placed = 0;
    let mut cursor = from;
    for id in task_ids {
        match place_task(state, id, date, cursor, DEFAULT_LEN) {
            Ok(b) => {
                cursor = b.start_min + b.duration_min;
                placed += 1;
            }
            Err(Error::NoSpace) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(placed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const DAY: Day = Day { start: 6 * 60, end: 24 * 60 };

    #[test]
    fn find_slot_uses_the_desired_spot_when_free() {
        assert_eq!(find_slot(&[], 9 * 60 + 7, 30, DAY), Some(9 * 60)); // 15 分刻みに切り捨てる
    }

    #[test]
    fn find_slot_skips_over_blocks() {
        // 9:00-10:00 と 10:00-10:30 が埋まっている
        let others = [(9 * 60, 60), (10 * 60, 30)];
        assert_eq!(find_slot(&others, 9 * 60 + 15, 30, DAY), Some(10 * 60 + 30));
        // 空きがその手前に入る長さなら、そこへ
        assert_eq!(find_slot(&[(9 * 60 + 30, 60)], 9 * 60, 30, DAY), Some(9 * 60));
        assert_eq!(find_slot(&[(9 * 60 + 30, 60)], 9 * 60, 60, DAY), Some(10 * 60 + 30));
    }

    #[test]
    fn find_slot_clamps_to_the_day_and_wraps_to_the_start() {
        assert_eq!(find_slot(&[], 3 * 60, 30, DAY), Some(6 * 60)); // 軸より前は軸の先頭へ
        // 軸の終わり近くで入らなければ、先頭から探し直す
        assert_eq!(find_slot(&[], 23 * 60 + 45, 30, DAY), Some(6 * 60));
        // 全部埋まっていれば None
        assert_eq!(find_slot(&[(6 * 60, 18 * 60)], 9 * 60, 30, DAY), None);
    }

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        (dir, s)
    }

    fn add_task(s: &AppState, title: &str) -> String {
        let t = crate::commands::tasks::create_task(s, crate::models::NewTask { title: title.into(), ..Default::default() }).unwrap();
        t.id
    }

    #[test]
    fn placing_moving_and_resizing_never_overlap() {
        let (_d, s) = state();
        let a = add_task(&s, "A");
        let b = add_task(&s, "B");
        let ba = place_task(&s, &a, "2026-10-05", 9 * 60, 60).unwrap();
        assert_eq!((ba.start_min, ba.duration_min), (9 * 60, 60));
        // B を A の上に置こうとすると、A の後ろへ詰める
        let bb = place_task(&s, &b, "2026-10-05", 9 * 60 + 15, 30).unwrap();
        assert_eq!(bb.start_min, 10 * 60);
        // A を伸ばしても、B の手前で止まる
        let ba2 = resize_block(&s, &ba.id, 4 * 60).unwrap();
        assert_eq!(ba2.duration_min, 60);
        // B を A に重なる位置へ動かしても重ならない（後ろの空きへ）
        let bb2 = move_block(&s, &bb.id, 9 * 60 + 30).unwrap();
        assert_eq!(bb2.start_min, 10 * 60);
        // 同じタスクを置き直すと、新しい棒は増えず、その棒が動く
        let again = place_task(&s, &b, "2026-10-05", 12 * 60, 30).unwrap();
        assert_eq!((again.id, again.start_min), (bb.id.clone(), 12 * 60));
        assert_eq!(s.data.lock().plans.len(), 2);
        // 外す
        remove_block(&s, &bb.id).unwrap();
        assert_eq!(s.data.lock().plans.len(), 1);
        assert!(remove_block(&s, &bb.id).is_err());
    }

    #[test]
    fn arrange_places_tasks_one_after_another() {
        let (_d, s) = state();
        let ids: Vec<String> = ["A", "B", "C"].iter().map(|t| add_task(&s, t)).collect();
        let n = arrange_from(&s, "2026-10-05", 13 * 60 + 5, &ids).unwrap();
        assert_eq!(n, 3);
        let starts: Vec<u16> = s.data.lock().plans.iter().map(|b| b.start_min).collect();
        assert_eq!(starts, [13 * 60, 13 * 60 + 30, 14 * 60]);
    }

    #[test]
    fn shifting_does_not_jump_over_neighbours_or_leave_the_day() {
        let (_d, s) = state();
        let a = add_task(&s, "A");
        let b = add_task(&s, "B");
        let ba = place_task(&s, &a, "2026-10-05", 9 * 60, 30).unwrap();
        let bb = place_task(&s, &b, "2026-10-05", 9 * 60 + 30, 30).unwrap();
        // B を上へ 15 分は A に当たるので動かない
        assert_eq!(shift_block(&s, &bb.id, -15).unwrap().start_min, 9 * 60 + 30);
        // 下へは動く
        assert_eq!(shift_block(&s, &bb.id, 15).unwrap().start_min, 9 * 60 + 45);
        // A を上へは動く。軸の先頭（6 時）を越えては動かない
        assert_eq!(shift_block(&s, &ba.id, -15).unwrap().start_min, 8 * 60 + 45);
        assert_eq!(shift_block(&s, &ba.id, -4 * 60).unwrap().start_min, 8 * 60 + 45);
    }

    #[test]
    fn old_files_without_plans_or_day_range_still_load() {
        let data = crate::seed::sample_data();
        let mut v = serde_json::to_value(&data).unwrap();
        v.as_object_mut().unwrap().remove("plans");
        let st = v["settings"].as_object_mut().unwrap();
        st.remove("dayStartHour");
        st.remove("dayEndHour");
        let back: AppData = serde_json::from_value(v).unwrap();
        assert!(back.plans.is_empty());
        assert_eq!((back.settings.day_start_hour, back.settings.day_end_hour), (6, 24));
    }
}
