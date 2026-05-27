import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import type { Project, Task } from '@/types';
import TaskRow from './TaskRow';

interface Props {
  title: string;
  subtitle?: string;
  tasks: Task[];
  project?: Project | null;
  onOpen: (id: string) => void;
}

export default function TaskTable({ title, subtitle, tasks, project, onOpen }: Props) {
  const addTask = useTaskStore((s) => s.addTask);
  const [composer, setComposer] = useState('');
  const [archiveOpen, setArchiveOpen] = useState(false);

  const activeTasks = tasks.filter((t) => t.status !== 'done');
  const doneTasks = tasks.filter((t) => t.status === 'done');

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!composer.trim()) return;
    await addTask({
      title: composer.trim(),
      projectId: project?.id ?? null,
    });
    setComposer('');
  };

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-border flex items-baseline gap-3">
        <h1 className="text-base font-semibold">{title}</h1>
        {subtitle && <span className="text-2xs text-mute font-mono">{subtitle}</span>}
        <span className="ml-auto text-2xs text-mute font-mono">{activeTasks.length} todo</span>
      </header>

      <form onSubmit={submit} className="px-6 py-2 border-b border-border">
        <input
          value={composer}
          onChange={(e) => setComposer(e.target.value)}
          placeholder="+ 新しいタスク (Enter で追加)"
          className="w-full bg-transparent outline-none text-sm placeholder:text-faint"
        />
      </form>

      <div className="flex-1 overflow-y-auto">
        {activeTasks.length === 0 && doneTasks.length === 0 ? (
          <div className="p-12 text-center text-2xs text-faint font-mono">
            タスクなし
          </div>
        ) : (
          <>
            {activeTasks.map((t) => <TaskRow key={t.id} task={t} onOpen={onOpen} />)}

            {doneTasks.length > 0 && (
              <div className="border-t border-border">
                <button
                  onClick={() => setArchiveOpen((v) => !v)}
                  className="w-full px-6 py-2 flex items-center gap-2 text-2xs font-mono text-mute hover:text-ink"
                >
                  <span>{archiveOpen ? '▼' : '▶'}</span>
                  <span>完了済み {doneTasks.length}</span>
                </button>
                {archiveOpen && (
                  <div className="opacity-60">
                    {doneTasks.map((t) => <TaskRow key={t.id} task={t} onOpen={onOpen} />)}
                  </div>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
