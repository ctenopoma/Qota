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
  const [filter, setFilter] = useState<'all' | 'todo' | 'done'>('all');

  const visible = tasks.filter((t) =>
    filter === 'all' ? true : filter === 'done' ? t.status === 'done' : t.status !== 'done',
  );
  const todoCount = tasks.filter((t) => t.status !== 'done').length;
  const doneCount = tasks.filter((t) => t.status === 'done').length;

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!composer.trim()) return;
    await addTask({
      title: composer.trim(),
      projectId: project?.id ?? null,
    });
    setComposer('');
  };

  const tabBtn = (active: boolean) =>
    `px-2 py-0.5 rounded text-2xs font-mono ${active ? 'bg-panel2 text-ink' : 'text-mute hover:text-ink'}`;

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-border flex items-baseline gap-3">
        <h1 className="text-base font-semibold">{title}</h1>
        {subtitle && <span className="text-2xs text-mute font-mono">{subtitle}</span>}
        <div className="ml-auto flex items-center gap-2">
          <button onClick={() => setFilter('all')} className={tabBtn(filter === 'all')}>
            ALL {tasks.length}
          </button>
          <button onClick={() => setFilter('todo')} className={tabBtn(filter === 'todo')}>
            TODO {todoCount}
          </button>
          <button onClick={() => setFilter('done')} className={tabBtn(filter === 'done')}>
            DONE {doneCount}
          </button>
        </div>
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
        {visible.length === 0 ? (
          <div className="p-12 text-center text-2xs text-faint font-mono">
            タスクなし
          </div>
        ) : (
          visible.map((t) => <TaskRow key={t.id} task={t} onOpen={onOpen} />)
        )}
      </div>
    </div>
  );
}
