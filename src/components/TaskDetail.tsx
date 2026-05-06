import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';

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
  const addSubtask = useTaskStore((s) => s.addSubtask);
  const toggleSubtask = useTaskStore((s) => s.toggleSubtask);
  const deleteSubtask = useTaskStore((s) => s.deleteSubtask);

  const [subInput, setSubInput] = useState('');

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
            value={t.title}
            onChange={(e) => updateTask(t.id, { title: e.target.value })}
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
              value={t.notes}
              onChange={(e) => updateTask(t.id, { notes: e.target.value })}
              rows={4}
              placeholder="メモを追加…"
              className="w-full bg-transparent border border-border rounded p-2 text-sm outline-none"
            />
          </div>

          <div>
            <div className="text-2xs font-mono uppercase tracking-widest text-faint mb-1">サブタスク</div>
            <div className="flex flex-col gap-0.5 mb-2">
              {t.subtasks.map((s) => (
                <div key={s.id} className="flex items-center gap-2 group">
                  <button
                    onClick={() => toggleSubtask(t.id, s.id)}
                    className={`w-3 h-3 rounded-sm border ${
                      s.done ? 'bg-accent border-accent' : 'border-mute'
                    }`}
                  />
                  <span className={`flex-1 text-sm ${s.done ? 'line-through text-mute' : ''}`}>
                    {s.title}
                  </span>
                  <button
                    onClick={() => deleteSubtask(t.id, s.id)}
                    className="text-2xs text-faint opacity-0 group-hover:opacity-100"
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

        <footer className="px-4 py-2 border-t border-border">
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
