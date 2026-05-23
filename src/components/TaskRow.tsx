import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { formatDue, isOverdue } from '@/lib/date';
import type { Priority, Task } from '@/types';

interface Props {
  task: Task;
  onOpen: (id: string) => void;
}

const priColor = (p: Priority): string => {
  if (p === 'high') return 'oklch(var(--c-high))';
  if (p === 'medium') return 'oklch(var(--c-med))';
  return 'oklch(var(--c-faint))';
};

const priLabel = (p: Priority): string => (p === 'high' ? 'H' : p === 'medium' ? 'M' : p === 'low' ? 'L' : '–');

export default function TaskRow({ task: t, onOpen }: Props) {
  const tags = useTaskStore((s) => s.tags);
  const projects = useTaskStore((s) => s.projects);
  const toggleTask = useTaskStore((s) => s.toggleTask);
  const updateTask = useTaskStore((s) => s.updateTask);
  const deleteTask = useTaskStore((s) => s.deleteTask);
  const duplicateTask = useTaskStore((s) => s.duplicateTask);
  const toggleSubtask = useTaskStore((s) => s.toggleSubtask);
  const startTimer = useTaskStore((s) => s.startTimer);
  const stopTimer = useTaskStore((s) => s.stopTimer);

  const [hover, setHover] = useState(false);
  const [editingDue, setEditingDue] = useState(false);
  const [priOpen, setPriOpen] = useState(false);
  const [showSubtasks, setShowSubtasks] = useState(false);
  const [editingEstimate, setEditingEstimate] = useState(false);

  const project = projects.find((p) => p.id === t.projectId);
  const taskTags = tags.filter((tag) => t.tagIds.includes(tag.id));

  const setPriority = async (p: Priority) => {
    await updateTask(t.id, { priority: p });
    setPriOpen(false);
  };

  return (
    <>
      <div
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        className="grid grid-cols-[24px_24px_1fr_auto_64px_24px] items-center gap-2 px-6 border-b border-border row-hover text-sm"
        style={{ paddingTop: 'var(--row-pad, 7px)', paddingBottom: 'var(--row-pad, 7px)' }}
      >
      <button
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          toggleTask(t.id);
        }}
        className={`w-3.5 h-3.5 rounded-sm border ${
          t.status === 'done' ? 'bg-accent border-accent' : 'border-mute hover:border-ink'
        }`}
        aria-label="toggle"
      />

      <div className="relative">
        <button
          onClick={() => setPriOpen((v) => !v)}
          className="font-mono text-2xs w-5 text-center"
          style={{ color: priColor(t.priority) }}
        >
          {priLabel(t.priority)}
        </button>
        {priOpen && (
          <div className="absolute z-10 top-6 left-0 bg-panel2 border border-border rounded shadow-lg flex flex-col">
            {(['high', 'medium', 'low', null] as Priority[]).map((p) => (
              <button
                key={p ?? 'none'}
                onClick={() => setPriority(p)}
                className="px-2 py-1 text-2xs text-left hover:bg-panel font-mono"
                style={{ color: priColor(p) }}
              >
                {priLabel(p)}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="flex items-center gap-2 truncate">
        <button onClick={() => onOpen(t.id)} className="text-left truncate hover:underline">
          <span className={t.status === 'done' ? 'line-through text-mute' : ''}>{t.title}</span>
        </button>
        {t.subtasks.length > 0 && (
          <button 
            onClick={() => setShowSubtasks(!showSubtasks)}
            className="text-2xs text-faint font-mono hover:text-ink shrink-0 flex items-center gap-1"
          >
            {showSubtasks ? '▼' : '▶'} {t.subtasks.filter((s) => s.done).length}/{t.subtasks.length}
          </button>
        )}
      </div>

      <div className="flex gap-1">
        {t.interval && (
          <span className="text-2xs font-mono px-1.5 py-0.5 rounded border border-border text-mute" title={`繰り返し: ${t.interval}`}>
            🔁
          </span>
        )}
        {t.estimatedMinutes != null && t.status === 'done' && t.actualMinutes != null && (
          <span
            className={`text-2xs font-mono px-1.5 py-0.5 rounded border ${
              t.actualMinutes <= t.estimatedMinutes
                ? 'border-accent/40 text-accent'
                : 'border-high/40 text-high'
            }`}
          >
            {t.actualMinutes <= t.estimatedMinutes
              ? `-${t.estimatedMinutes - t.actualMinutes}m`
              : `+${t.actualMinutes - t.estimatedMinutes}m`}
          </span>
        )}
        {t.startedAt && (
          <button
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              stopTimer(t.id);
            }}
            className="text-2xs font-mono px-1.5 py-0.5 rounded border border-accent/40 text-accent animate-pulse hover:border-accent hover:bg-accent/10"
            title="計測を停止"
          >
            ⏹ 計測中
          </button>
        )}
        {t.estimatedMinutes != null && t.status !== 'done' && !t.startedAt && (
          <button
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              startTimer(t.id);
            }}
            className="text-2xs font-mono px-1.5 py-0.5 rounded border border-border text-mute hover:text-ink hover:border-mute flex items-center gap-1"
            title="計測を開始"
          >
            <span>▶</span>
            <span>{t.estimatedMinutes >= 60 ? `${t.estimatedMinutes / 60}h` : `${t.estimatedMinutes}m`}</span>
          </button>
        )}
        {t.estimatedMinutes == null && t.status !== 'done' && (
          editingEstimate ? (
            <select
              autoFocus
              className="bg-transparent border border-border rounded px-1 py-0.5 text-2xs font-mono w-16"
              onChange={(e) => {
                if (e.target.value) {
                  updateTask(t.id, { estimatedMinutes: Number(e.target.value) });
                }
                setEditingEstimate(false);
              }}
              onBlur={() => setEditingEstimate(false)}
            >
              <option value="">-</option>
              {[30, 60, 90, 120, 150, 180, 240, 300, 360, 420, 480].map((m) => (
                <option className="bg-panel text-ink" key={m} value={m}>
                  {m >= 60 ? `${m / 60}h` : `${m}m`}
                </option>
              ))}
            </select>
          ) : (
            <button
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setEditingEstimate(true);
              }}
              className={`text-2xs font-mono px-1.5 py-0.5 rounded border border-transparent text-faint hover:text-ink ${hover ? 'opacity-100' : 'opacity-0'}`}
              title="見積もりを追加"
            >
              ⏱
            </button>
          )
        )}
        {project && (
          <span className="text-2xs font-mono px-1.5 py-0.5 rounded border border-border text-mute">
            {project.name}
          </span>
        )}
        {taskTags.map((tag) => (
          <span key={tag.id} className="text-2xs font-mono text-faint">
            #{tag.name}
          </span>
        ))}
      </div>

      {editingDue ? (
        <input
          type="date"
          autoFocus
          defaultValue={t.due ?? ''}
          onBlur={(e) => {
            updateTask(t.id, { due: e.target.value || null });
            setEditingDue(false);
          }}
          className="bg-transparent border border-border rounded text-2xs font-mono px-1 w-16"
        />
      ) : (
        <button
          onClick={() => setEditingDue(true)}
          className={`text-2xs font-mono text-right ${
            isOverdue(t.due) && t.status !== 'done' ? 'text-high' : 'text-mute'
          }`}
        >
          {formatDue(t.due) || '–'}
        </button>
      )}

      <div className={`flex items-center gap-2 ${hover ? 'opacity-100' : 'opacity-0'}`}>
        <button
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            duplicateTask(t.id);
          }}
          className="text-2xs text-faint hover:text-ink"
          title="複製"
        >
          ⧉
        </button>
        <button
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (confirm('削除しますか？')) deleteTask(t.id);
          }}
          className="text-2xs text-faint hover:text-high"
          title="削除"
        >
          ×
        </button>
      </div>
      </div>

      {showSubtasks && t.subtasks.length > 0 && (
        <div className="bg-panel/50 px-6 py-2 border-b border-border pl-[80px]">
          {t.subtasks.map((s) => (
            <div key={s.id} className="flex items-center gap-2 py-0.5 text-sm">
              <button
                onClick={() => toggleSubtask(t.id, s.id)}
                className={`w-3 h-3 rounded-sm border shrink-0 ${s.done ? 'bg-accent border-accent' : 'border-mute hover:border-ink'}`}
                aria-label="toggle subtask"
              />
              <span className={s.done ? 'line-through text-mute' : 'text-faint'}>{s.title}</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
