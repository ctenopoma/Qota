import { useTaskStore } from '@/store/useTaskStore';
import { isOverdue, isToday, isThisWeek, formatDue, sortByDue } from '@/lib/date';
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

export default function Upcoming({ onOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const projects = useTaskStore((s) => s.projects);

  const buckets: { key: string; label: string; test: (t: Task) => boolean }[] = [
    { key: 'overdue', label: '期限切れ', test: (t) => isOverdue(t.due) && t.status !== 'done' },
    { key: 'today', label: '今日', test: (t) => isToday(t.due) },
    { key: 'week', label: '今週', test: (t) => isThisWeek(t.due) && !isToday(t.due) && !isOverdue(t.due) },
    { key: 'later', label: 'それ以降', test: (t) => !!t.due && !isThisWeek(t.due) && !isOverdue(t.due) },
    { key: 'someday', label: '日付なし', test: (t) => !t.due },
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
                    <button
                      key={t.id}
                      onClick={() => onOpen(t.id)}
                      className="text-left text-sm bg-bg border border-border rounded px-2 py-1.5 hover:border-mute"
                    >
                      <div className={t.status === 'done' ? 'line-through text-mute' : ''}>
                        {t.title}
                      </div>
                      <div className="flex items-center justify-between mt-0.5">
                        {project && (
                          <span className="text-2xs font-mono px-1 py-0.5 rounded border border-border text-mute inline-flex items-center gap-1">
                            <span
                              className="w-1.5 h-1.5 rounded-full shrink-0"
                              style={{ background: project.color }}
                            />
                            {project.name}
                          </span>
                        )}
                        {t.due && (
                          <div className="text-2xs font-mono text-faint">
                            {formatDue(t.due)}
                          </div>
                        )}
                      </div>
                    </button>
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
