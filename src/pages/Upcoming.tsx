import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { isOverdue, isToday, isTomorrow, isThisWeek, isThisMonth, isNextMonth, formatDue, sortByDue } from '@/lib/date';
import type { Priority, Task } from '@/types';

interface Props {
  onOpen: (id: string) => void;
}

const PRIORITY_ORDER: Record<string, number> = {
  high: 0,
  medium: 1,
  low: 2,
  null: 3,
};

const sortByPriority = (tasks: Task[]): Task[] => {
  return [...tasks].sort((a, b) => {
    const pa = PRIORITY_ORDER[a.priority ?? 'null'];
    const pb = PRIORITY_ORDER[b.priority ?? 'null'];
    return pa - pb;
  });
};

function UpcomingTaskCard({ t, project, onOpen }: { t: Task; project: any; onOpen: (id: string) => void }) {
  const [showSubtasks, setShowSubtasks] = useState(false);
  const toggleSubtask = useTaskStore((s) => s.toggleSubtask);

  return (
    <div className="flex flex-col bg-bg border border-border rounded hover:border-mute overflow-hidden">
      <div
        className="text-left text-sm px-2 py-1.5 cursor-pointer"
        onClick={() => onOpen(t.id)}
      >
        <div className="flex items-start justify-between gap-1">
          <div className={t.status === 'done' ? 'line-through text-mute truncate' : 'truncate'}>
            {t.title}
          </div>
          {t.subtasks.length > 0 && (
            <button 
              onClick={(e) => { e.stopPropagation(); setShowSubtasks(!showSubtasks); }}
              className="text-2xs text-faint font-mono hover:text-ink shrink-0 flex items-center gap-0.5 mt-0.5"
            >
              {showSubtasks ? '▼' : '▶'} {t.subtasks.filter((s) => s.done).length}/{t.subtasks.length}
            </button>
          )}
        </div>
        <div className="flex items-center justify-between mt-0.5">
          {project ? (
            <span className="text-2xs font-mono px-1 py-0.5 rounded border border-border text-mute inline-flex items-center gap-1">
              <span
                className="w-1.5 h-1.5 rounded-full shrink-0"
                style={{ background: project.color }}
              />
              {project.name}
            </span>
          ) : (
            <div />
          )}
          {t.due && (
            <div className="text-2xs font-mono text-faint ml-auto">
              {formatDue(t.due)}
            </div>
          )}
        </div>
      </div>
      {showSubtasks && t.subtasks.length > 0 && (
        <div className="bg-panel/50 px-2 py-1.5 border-t border-border">
          {t.subtasks.map((s) => (
            <div key={s.id} className="flex items-center gap-1.5 py-0.5 text-xs">
              <button
                onClick={() => toggleSubtask(t.id, s.id)}
                className={`w-3 h-3 rounded-sm border shrink-0 ${s.done ? 'bg-accent border-accent' : 'border-mute hover:border-ink'}`}
                aria-label="toggle subtask"
              />
              <span className={`truncate ${s.done ? 'line-through text-mute' : 'text-faint'}`}>{s.title}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function Upcoming({ onOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const projects = useTaskStore((s) => s.projects);

  const buckets: { key: string; label: string; test: (t: Task) => boolean }[] = [
    { key: 'overdue', label: '期限切れ', test: (t) => isOverdue(t.due) && t.status !== 'done' },
    { key: 'today', label: '今日', test: (t) => isToday(t.due) },
    { key: 'tomorrow', label: '明日', test: (t) => isTomorrow(t.due) },
    { key: 'week', label: '今週', test: (t) => isThisWeek(t.due) && !isToday(t.due) && !isTomorrow(t.due) && !isOverdue(t.due) },
    { key: 'month', label: '今月', test: (t) => isThisMonth(t.due) && !isThisWeek(t.due) && !isToday(t.due) && !isTomorrow(t.due) && !isOverdue(t.due) },
    { key: 'next_month', label: '来月', test: (t) => isNextMonth(t.due) && !isThisWeek(t.due) && !isTomorrow(t.due) },
    { key: 'later', label: 'それ以降', test: (t) => !!t.due && !isThisMonth(t.due) && !isNextMonth(t.due) && !isThisWeek(t.due) && !isOverdue(t.due) },
    { key: 'someday', label: '記載なし', test: (t) => !t.due },
  ];

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-border">
        <h1 className="text-base font-semibold">今後</h1>
      </header>
      <div className="flex-1 overflow-x-auto p-4 grid grid-flow-col auto-cols-[280px] gap-3">
        {buckets.map((b) => {
          const items = sortByPriority(sortByDue(tasks.filter(b.test)));
          return (
            <div key={b.key} className="bg-panel border border-border rounded flex flex-col">
              <div className="px-3 py-2 border-b border-border flex justify-between text-2xs font-mono uppercase tracking-widest">
                <span>{b.label}</span>
                <span className="text-faint">{items.length}</span>
              </div>
              <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-1">
                {items.length === 0 && (
                  <div className="text-2xs text-faint text-center py-4 font-mono">なし</div>
                )}
                {items.map((t) => {
                  const project = projects.find((p) => p.id === t.projectId);
                  return (
                    <UpcomingTaskCard key={t.id} t={t} project={project} onOpen={onOpen} />
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
