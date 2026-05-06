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

  const [hover, setHover] = useState(false);
  const [editingDue, setEditingDue] = useState(false);
  const [priOpen, setPriOpen] = useState(false);

  const project = projects.find((p) => p.id === t.projectId);
  const taskTags = tags.filter((tag) => t.tagIds.includes(tag.id));

  const setPriority = async (p: Priority) => {
    await updateTask(t.id, { priority: p });
    setPriOpen(false);
  };

  return (
    <div
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      className="grid grid-cols-[24px_24px_1fr_auto_64px_24px] items-center gap-2 px-6 border-b border-border row-hover text-sm"
      style={{ paddingTop: 'var(--row-pad, 7px)', paddingBottom: 'var(--row-pad, 7px)' }}
    >
      <button
        onClick={() => toggleTask(t.id)}
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

      <button onClick={() => onOpen(t.id)} className="text-left truncate">
        <span className={t.status === 'done' ? 'line-through text-mute' : ''}>{t.title}</span>
        {t.subtasks.length > 0 && (
          <span className="ml-2 text-2xs text-faint font-mono">
            ⌘ {t.subtasks.filter((s) => s.done).length}/{t.subtasks.length}
          </span>
        )}
      </button>

      <div className="flex gap-1">
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

      <button
        onClick={() => {
          if (confirm('削除しますか？')) deleteTask(t.id);
        }}
        className={`text-2xs text-faint hover:text-high ${hover ? 'opacity-100' : 'opacity-0'}`}
      >
        ×
      </button>
    </div>
  );
}
