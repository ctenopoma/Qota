import { useState, useEffect } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { getEffectiveEstimatedMinutes } from '@/lib/time';

const SUBTASK_TIME_OPTIONS = [5, 10, 15, 30, 45, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330, 360];

interface Props {
  id: string;
  onClose: () => void;
}

export default function TaskDetail({ id, onClose }: Props) {
  const task = useTaskStore((s) => s.tasks.find((t) => t.id === id));
  const projects = useTaskStore((s) => s.projects);
  const tags = useTaskStore((s) => s.tags);
  const updateTask = useTaskStore((s) => s.updateTask);
  const deleteTask = useTaskStore((s) => s.deleteTask);
  const duplicateTask = useTaskStore((s) => s.duplicateTask);
  const addSubtask = useTaskStore((s) => s.addSubtask);
  const toggleSubtask = useTaskStore((s) => s.toggleSubtask);
  const deleteSubtask = useTaskStore((s) => s.deleteSubtask);
  const updateSubtask = useTaskStore((s) => s.updateSubtask);
  const moveSubtask = useTaskStore((s) => s.moveSubtask);
  const startTimer = useTaskStore((s) => s.startTimer);
  const stopTimer = useTaskStore((s) => s.stopTimer);

  const [subInput, setSubInput] = useState('');
  const [titleInput, setTitleInput] = useState(task?.title ?? '');
  const [notesInput, setNotesInput] = useState(task?.notes ?? '');
  const [editingSubtask, setEditingSubtask] = useState<{ id: string; title: string } | null>(null);
  const [elapsedDisplay, setElapsedDisplay] = useState('');
  const [subtaskElapsed, setSubtaskElapsed] = useState<{ [id: string]: string }>({});
  const startSubtaskTimer = useTaskStore((s) => s.startSubtaskTimer);
  const stopSubtaskTimer = useTaskStore((s) => s.stopSubtaskTimer);

  useEffect(() => {
    if (task) {
      setTitleInput(task.title);
      setNotesInput(task.notes);
    }
  }, [task?.id]);

  useEffect(() => {
    if (!task?.startedAt) { setElapsedDisplay(''); return; }
    const tick = () => {
      const mins = Math.floor((Date.now() - new Date(task.startedAt!).getTime()) / 60000);
      const h = Math.floor(mins / 60);
      const m = mins % 60;
      setElapsedDisplay(h > 0 ? `${h}h ${m}m` : `${m}m`);
    };
    tick();
    const iv = setInterval(tick, 15000);
    return () => clearInterval(iv);
  }, [task?.startedAt]);

  useEffect(() => {
    if (!task) return;
    const running = task.subtasks.filter(s => s.startedAt);
    if (running.length === 0) { setSubtaskElapsed({}); return; }

    const tick = () => {
      const elapsed: { [id: string]: string } = {};
      for (const s of running) {
        if (!s.startedAt) continue;
        const mins = Math.floor((Date.now() - new Date(s.startedAt).getTime()) / 60000);
        const h = Math.floor(mins / 60);
        const m = mins % 60;
        elapsed[s.id] = h > 0 ? `${h}h ${m}m` : `${m}m`;
      }
      setSubtaskElapsed(elapsed);
    };
    tick();
    const iv = setInterval(tick, 15000);
    return () => clearInterval(iv);
  }, [task?.subtasks]);

  if (!task) {
    return null;
  }

  const t = task;

  return (
    <div className="fixed inset-0 bg-black/40 grid place-items-end z-40" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="h-full w-[480px] bg-panel border-l border-border flex flex-col"
      >
        <header className="px-4 py-3 border-b border-border flex items-center gap-2">
          <span className="text-2xs font-mono uppercase tracking-widest text-faint">タスク詳細</span>
          <button onClick={onClose} className="ml-auto text-2xs text-faint hover:text-ink">
            ×
          </button>
        </header>

        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
          <input
            value={titleInput}
            onChange={(e) => setTitleInput(e.target.value)}
            onBlur={() => {
              if (titleInput !== t.title && titleInput.trim()) {
                updateTask(t.id, { title: titleInput });
              }
            }}
            className="bg-transparent text-base font-semibold outline-none"
          />

          <div className="grid grid-cols-[80px_1fr] gap-2 text-sm">
            <span className="text-mute text-2xs font-mono pt-1">状態</span>
            <button
              onClick={() => updateTask(t.id, { status: t.status === 'done' ? 'todo' : 'done' })}
              className="text-left text-2xs font-mono"
            >
              {t.status}
            </button>

            <span className="text-mute text-2xs font-mono pt-1">優先度</span>
            <select
              value={t.priority ?? ''}
              onChange={(e) => updateTask(t.id, { priority: (e.target.value || null) as any })}
              className="bg-transparent border border-border rounded px-1 text-2xs font-mono"
            >
              <option className="bg-panel text-ink" value="">–</option>
              <option className="bg-panel text-ink" value="high">high</option>
              <option className="bg-panel text-ink" value="medium">medium</option>
              <option className="bg-panel text-ink" value="low">low</option>
            </select>

            <span className="text-mute text-2xs font-mono pt-1">期日</span>
            <input
              type="date"
              value={t.due ?? ''}
              onChange={(e) => updateTask(t.id, { due: e.target.value || null })}
              className="bg-transparent border border-border rounded px-1 text-2xs font-mono"
            />

            <span className="text-mute text-2xs font-mono pt-1">繰り返し</span>
            <div className="flex flex-col gap-2 text-sm">
              <label className="flex items-center gap-2 cursor-pointer w-fit">
                <input
                  type="checkbox"
                  checked={!!t.interval}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateTask(t.id, { interval: 'daily' });
                    } else {
                      updateTask(t.id, { interval: null });
                    }
                  }}
                  className="accent-accent cursor-pointer"
                />
                <span className="text-xs">繰り返しを有効にする</span>
              </label>

              {!!t.interval && (
                <div className="flex flex-col gap-2 pl-6 border-l-2 border-border/50">
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-mute w-12">頻度</span>
                    <select
                      value={t.interval}
                      onChange={(e) => updateTask(t.id, { interval: e.target.value as any })}
                      className="bg-transparent border border-border rounded px-1 text-2xs font-mono flex-1"
                    >
                      <option className="bg-panel text-ink" value="daily">毎日</option>
                      <option className="bg-panel text-ink" value="weekdays">平日</option>
                      <option className="bg-panel text-ink" value="weekly">毎週</option>
                      <option className="bg-panel text-ink" value="monthly">毎月</option>
                      <option className="bg-panel text-ink" value="yearly">毎年</option>
                    </select>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-mute w-12">終了日</span>
                    <input
                      type="date"
                      value={t.recurrenceEnd ?? ''}
                      onChange={(e) => updateTask(t.id, { recurrenceEnd: e.target.value || null })}
                      className="bg-transparent border border-border rounded px-1 text-2xs font-mono flex-1"
                      title="この日付までのタスクが一括で生成されます"
                    />
                  </div>
                </div>
              )}
            </div>

            <span className="text-mute text-2xs font-mono pt-1">見積もり</span>
            <div className="flex flex-col gap-2 text-sm">
              {(() => {
                const effective = getEffectiveEstimatedMinutes(t);
                const hasSubtaskEstimates = t.subtasks.some(s => s.estimatedMinutes != null);
                return effective == null ? (
                  <button
                    onClick={() => updateTask(t.id, { estimatedMinutes: 30 })}
                    className="text-left text-2xs font-mono text-faint hover:text-ink"
                  >
                    ⏱ 見積もりを追加
                  </button>
                ) : (
                  <div className="flex items-center gap-2">
                    <select
                      value={t.estimatedMinutes ?? ''}
                      onChange={(e) =>
                        updateTask(t.id, { estimatedMinutes: Number(e.target.value) || null })
                      }
                      disabled={hasSubtaskEstimates}
                      className={`bg-transparent border border-border rounded px-1 text-2xs font-mono ${
                        hasSubtaskEstimates ? 'opacity-50 cursor-not-allowed' : ''
                      }`}
                      title={hasSubtaskEstimates ? 'サブタスクの合計時間が使用されています' : ''}
                    >
                      {[30, 60, 90, 120, 150, 180, 240, 300, 360, 420, 480].map((m) => (
                        <option className="bg-panel text-ink" key={m} value={m}>
                          {m >= 60 ? `${m / 60}h` : `${m}m`}
                        </option>
                      ))}
                    </select>
                    {!hasSubtaskEstimates && (
                      <button
                        onClick={() => updateTask(t.id, { estimatedMinutes: null })}
                        className="text-2xs text-faint hover:text-high"
                      >
                        ×
                      </button>
                    )}
                    {hasSubtaskEstimates && (
                      <span className="text-2xs text-faint">(サブタスク合計)</span>
                    )}
                  </div>
                );
              })()}

              {t.estimatedMinutes != null && (
                <div className="flex flex-col gap-1.5 pl-6 border-l-2 border-border/50">
                  {/* Timer */}
                  {t.status !== 'done' && (
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-mute w-12">計測</span>
                      {t.startedAt ? (
                        <>
                          <span className="text-2xs font-mono text-accent animate-pulse">
                            ● {elapsedDisplay || '0m'}
                          </span>
                          <button
                            onClick={() => stopTimer(t.id)}
                            className="text-2xs font-mono text-faint hover:text-ink border border-border rounded px-1.5 py-0.5"
                          >
                            ⏹ 停止
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => startTimer(t.id)}
                          className="text-2xs font-mono text-faint hover:text-ink border border-border rounded px-1.5 py-0.5"
                        >
                          ▶ 開始
                        </button>
                      )}
                    </div>
                  )}
                  {/* Actual time */}
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-mute w-12">実績</span>
                    <input
                      type="number"
                      min="0"
                      step="5"
                      value={t.actualMinutes ?? ''}
                      onChange={(e) =>
                        updateTask(t.id, {
                          actualMinutes: e.target.value ? Number(e.target.value) : null,
                        })
                      }
                      placeholder="分"
                      className="bg-transparent border border-border rounded px-1 text-2xs font-mono w-16"
                    />
                    <span className="text-2xs text-faint">分</span>
                    {t.status === 'done' && t.actualMinutes != null && (
                      <span
                        className={`text-2xs font-mono ${
                          t.actualMinutes <= t.estimatedMinutes! ? 'text-accent' : 'text-high'
                        }`}
                      >
                        {t.actualMinutes <= t.estimatedMinutes!
                          ? `${t.estimatedMinutes! - t.actualMinutes}分 短縮 🎉`
                          : `${t.actualMinutes - t.estimatedMinutes!}分 オーバー`}
                      </span>
                    )}
                  </div>
                </div>
              )}
            </div>

            <span className="text-mute text-2xs font-mono pt-1">プロジェクト</span>
            <select
              value={t.projectId ?? ''}
              onChange={(e) => updateTask(t.id, { projectId: e.target.value || null })}
              className="bg-transparent border border-border rounded px-1 text-2xs font-mono"
            >
              <option className="bg-panel text-ink" value="">受信箱</option>
              {projects.map((p) => (
                <option className="bg-panel text-ink" key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>

            <span className="text-mute text-2xs font-mono pt-1">タグ</span>
            <div className="flex flex-wrap gap-1">
              {tags.map((tag) => {
                const on = t.tagIds.includes(tag.id);
                return (
                  <button
                    key={tag.id}
                    onClick={() =>
                      updateTask(t.id, {
                        tagIds: on ? t.tagIds.filter((x) => x !== tag.id) : [...t.tagIds, tag.id],
                      })
                    }
                    className={`text-2xs font-mono px-1.5 py-0.5 rounded border ${
                      on ? 'border-accent text-accent' : 'border-border text-mute'
                    }`}
                  >
                    #{tag.name}
                  </button>
                );
              })}
            </div>
          </div>

          <div>
            <div className="text-2xs font-mono uppercase tracking-widest text-faint mb-1">メモ</div>
            <textarea
              value={notesInput}
              onChange={(e) => setNotesInput(e.target.value)}
              onBlur={() => {
                if (notesInput !== t.notes) {
                  updateTask(t.id, { notes: notesInput });
                }
              }}
              rows={4}
              placeholder="メモを追加…"
              className="w-full bg-transparent border border-border rounded p-2 text-sm outline-none"
            />
          </div>

          <div>
            <div className="text-2xs font-mono uppercase tracking-widest text-faint mb-1">サブタスク</div>
            <div className="flex flex-col gap-0.5 mb-2">
              {t.subtasks.map((s, i) => (
                <div key={s.id} className="flex items-center gap-1 group">
                  <div className="flex flex-col gap-0.5">
                    <button
                      onClick={() => moveSubtask(t.id, s.id, 'up')}
                      disabled={i === 0}
                      className="text-2xs text-faint disabled:opacity-30 hover:opacity-100"
                      title="上に移動"
                    >
                      ▲
                    </button>
                    <button
                      onClick={() => moveSubtask(t.id, s.id, 'down')}
                      disabled={i === t.subtasks.length - 1}
                      className="text-2xs text-faint disabled:opacity-30 hover:opacity-100"
                      title="下に移動"
                    >
                      ▼
                    </button>
                  </div>
                  <button
                    onClick={() => toggleSubtask(t.id, s.id)}
                    className={`w-3 h-3 rounded-sm border shrink-0 ${
                      s.done ? 'bg-accent border-accent' : 'border-mute'
                    }`}
                  />
                  {editingSubtask?.id === s.id ? (
                    <input
                      value={editingSubtask.title}
                      onChange={(e) => setEditingSubtask({ ...editingSubtask, title: e.target.value })}
                      onBlur={() => {
                        if (editingSubtask.title.trim() && editingSubtask.title !== s.title) {
                          updateSubtask(t.id, s.id, { title: editingSubtask.title });
                        }
                        setEditingSubtask(null);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') e.currentTarget.blur();
                        if (e.key === 'Escape') setEditingSubtask(null);
                      }}
                      autoFocus
                      className="flex-1 text-sm bg-transparent outline-none border-b border-border"
                    />
                  ) : (
                    <span
                      onClick={() => setEditingSubtask({ id: s.id, title: s.title })}
                      className={`flex-1 text-sm cursor-text ${s.done ? 'line-through text-mute' : ''}`}
                    >
                      {s.title}
                    </span>
                  )}
                  {/* Time selector */}
                  {s.estimatedMinutes == null ? (
                    <button
                      onClick={() => updateSubtask(t.id, s.id, { estimatedMinutes: 30 })}
                      className="text-2xs text-faint hover:text-ink shrink-0"
                      title="時間を追加"
                    >
                      ⊕
                    </button>
                  ) : (
                    <select
                      value={s.estimatedMinutes}
                      onChange={(e) =>
                        updateSubtask(t.id, s.id, { estimatedMinutes: Number(e.target.value) })
                      }
                      className="bg-transparent border border-border rounded px-1 text-2xs font-mono shrink-0"
                    >
                      {SUBTASK_TIME_OPTIONS.map((m) => (
                        <option className="bg-panel text-ink" key={m} value={m}>
                          {m >= 60 ? `${m / 60}h` : `${m}m`}
                        </option>
                      ))}
                    </select>
                  )}
                  {/* Actual time display */}
                  {s.actualMinutes != null && s.actualMinutes > 0 && (
                    <span
                      className={`text-2xs font-mono shrink-0 ${
                        s.estimatedMinutes != null && s.actualMinutes <= s.estimatedMinutes
                          ? 'text-accent'
                          : s.estimatedMinutes != null
                          ? 'text-high'
                          : 'text-faint'
                      }`}
                      title="実績時間"
                    >
                      実: {s.actualMinutes >= 60 ? `${Math.floor(s.actualMinutes / 60)}h${s.actualMinutes % 60 ? ` ${s.actualMinutes % 60}m` : ''}` : `${s.actualMinutes}m`}
                    </span>
                  )}
                  {/* Timer */}
                  {s.estimatedMinutes != null && t.status !== 'done' && (
                    <>
                      {s.startedAt ? (
                        <>
                          <span className="text-2xs font-mono text-accent animate-pulse shrink-0">
                            ● {subtaskElapsed[s.id] || '0m'}
                          </span>
                          <button
                            onClick={() => stopSubtaskTimer(t.id, s.id)}
                            className="text-2xs font-mono text-faint hover:text-ink border border-border rounded px-1.5 py-0.5 shrink-0"
                          >
                            ⏹
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => startSubtaskTimer(t.id, s.id)}
                          className="text-2xs font-mono text-faint hover:text-ink border border-border rounded px-1.5 py-0.5 shrink-0"
                        >
                          ▶
                        </button>
                      )}
                    </>
                  )}
                  <button
                    onClick={() => deleteSubtask(t.id, s.id)}
                    className="text-2xs text-faint opacity-0 group-hover:opacity-100 shrink-0"
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (!subInput.trim()) return;
                addSubtask(t.id, subInput.trim());
                setSubInput('');
              }}
            >
              <input
                value={subInput}
                onChange={(e) => setSubInput(e.target.value)}
                placeholder="+ サブタスクを追加"
                className="w-full bg-transparent border border-border rounded px-2 py-1 text-2xs outline-none"
              />
            </form>
          </div>
        </div>

        <footer className="px-4 py-2 border-t border-border flex items-center justify-between">
          <button
            onClick={() => {
              duplicateTask(t.id);
              onClose();
            }}
            className="text-2xs font-mono text-faint hover:text-ink hover:underline"
          >
            タスクを複製
          </button>
          <button
            onClick={() => {
              if (confirm('削除しますか？')) {
                deleteTask(t.id);
                onClose();
              }
            }}
            className="text-2xs font-mono text-high hover:underline"
          >
            タスクを削除
          </button>
        </footer>
      </div>
    </div>
  );
}
