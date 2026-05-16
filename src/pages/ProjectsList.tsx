import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { formatDue } from '@/lib/date';
import type { Priority } from '@/types';

const priColor = (p: Priority): string => {
  if (p === 'high') return 'oklch(var(--c-high))';
  if (p === 'medium') return 'oklch(var(--c-med))';
  return 'oklch(var(--c-faint))';
};

const priLabel = (p: Priority): string => (p === 'high' ? 'H' : p === 'medium' ? 'M' : p === 'low' ? 'L' : '–');

const PROJECT_COLORS = [
  '#ef4444', '#f97316', '#eab308', '#22c55e',
  '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899',
];

export default function ProjectsList({ onOpen }: { onOpen: (id: string) => void }) {
  const projects = useTaskStore((s) => s.projects);
  const tasks = useTaskStore((s) => s.tasks);
  const addProject = useTaskStore((s) => s.addProject);
  const deleteProject = useTaskStore((s) => s.deleteProject);
  const [name, setName] = useState('');
  const [selectedColor, setSelectedColor] = useState(PROJECT_COLORS[0]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    await addProject({ name: name.trim(), color: selectedColor });
    setName('');
    setSelectedColor(PROJECT_COLORS[0]);
  };

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <header className="px-6 py-4 border-b border-border">
        <h1 className="text-base font-semibold">プロジェクト</h1>
      </header>
      <form onSubmit={submit} className="px-6 py-3 border-b border-border flex-shrink-0 space-y-2">
        <div className="flex gap-2">
          {PROJECT_COLORS.map((color) => (
            <button
              key={color}
              type="button"
              onClick={() => setSelectedColor(color)}
              className={`w-5 h-5 rounded-full transition-transform ${
                selectedColor === color ? 'ring-2 ring-offset-2 ring-ink scale-125' : ''
              }`}
              style={{ background: color }}
            />
          ))}
        </div>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="+ 新しいプロジェクト"
          className="w-full bg-transparent outline-none text-sm placeholder:text-faint"
        />
      </form>
      <div className="flex-1 overflow-x-auto overflow-y-hidden p-6 flex gap-6 items-start">
        {projects.map((p) => {
          const projectTasks = tasks.filter((t) => t.projectId === p.id);
          const done = projectTasks.filter((t) => t.status === 'done').length;
          return (
            <div
              key={p.id}
              className="flex flex-col gap-3 w-72 flex-shrink-0 bg-panel border border-border rounded-lg p-3 max-h-full"
            >
              <div className="flex items-center justify-between px-1">
                <div className="flex items-center gap-2" style={{ cursor: 'pointer' }} onClick={() => onOpen(p.id)}>
                  <span
                    className="w-2 h-2 rounded-full shrink-0"
                    style={{ background: p.color }}
                  />
                  <span className="font-mono text-sm font-semibold truncate hover:underline">{p.name}</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-2xs text-faint font-mono">{done}/{projectTasks.length}</span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (confirm(`${p.name} を削除しますか？`)) deleteProject(p.id);
                    }}
                    className="text-faint hover:text-high"
                  >
                    ×
                  </button>
                </div>
              </div>
              <div className="flex-1 overflow-y-auto flex flex-col gap-2 pr-1" style={{ scrollbarWidth: 'thin' }}>
                {projectTasks.length === 0 ? (
                  <div className="text-2xs text-faint font-mono text-center py-4">タスクなし</div>
                ) : (
                  projectTasks.map((t) => (
                    <div
                      key={t.id}
                      onClick={() => onOpen(t.id)}
                      className="bg-bg border border-border rounded p-3 text-sm cursor-pointer hover:border-high transition-colors flex flex-col gap-2 shadow-sm"
                    >
                      <div className="flex justify-between items-start gap-2">
                        <span className={`leading-tight ${t.status === 'done' ? 'line-through text-mute' : ''}`}>
                          {t.title}
                        </span>
                        {t.priority !== null && t.priority !== undefined && (
                          <span className="text-2xs font-mono font-bold shrink-0 mt-0.5" style={{ color: priColor(t.priority) }}>
                            {priLabel(t.priority)}
                          </span>
                        )}
                      </div>
                      <div className="flex justify-between items-center text-2xs text-faint font-mono mt-1">
                        <span>{t.due ? formatDue(t.due) : ''}</span>
                        {t.subtasks.length > 0 && (
                          <span>
                            ⌘ {t.subtasks.filter((st) => st.done).length}/{t.subtasks.length}
                          </span>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
