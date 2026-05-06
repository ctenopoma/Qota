import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { formatDue } from '@/lib/date';
import type { Priority } from '@/types';

interface Props {
  onOpen: (id: string) => void;
}

const priColor = (p: Priority): string => {
  if (p === 'high') return 'oklch(var(--c-high))';
  if (p === 'medium') return 'oklch(var(--c-med))';
  return 'oklch(var(--c-faint))';
};

const priLabel = (p: Priority): string => (p === 'high' ? 'H' : p === 'medium' ? 'M' : p === 'low' ? 'L' : '–');

export default function Tags({ onOpen }: Props) {
  const tags = useTaskStore((s) => s.tags);
  const tasks = useTaskStore((s) => s.tasks);
  const addTag = useTaskStore((s) => s.addTag);
  const deleteTag = useTaskStore((s) => s.deleteTag);
  const [name, setName] = useState('');

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const clean = name.trim().replace(/^#/, '');
    if (!clean) return;
    await addTag(clean);
    setName('');
  };

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <header className="px-6 py-4 border-b border-border">
        <h1 className="text-base font-semibold">タグ</h1>
      </header>
      <form onSubmit={submit} className="px-6 py-2 border-b border-border flex-shrink-0">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="+ 新しいタグ"
          className="w-full bg-transparent outline-none text-sm placeholder:text-faint"
        />
      </form>
      <div className="flex-1 overflow-x-auto overflow-y-hidden p-6 flex gap-6 items-start">
        {tags.map((tag) => {
          const tagTasks = tasks.filter((t) => t.tagIds.includes(tag.id));
          return (
            <div
              key={tag.id}
              className="flex flex-col gap-3 w-72 flex-shrink-0 bg-panel border border-border rounded-lg p-3 max-h-full"
            >
              <div className="flex items-center justify-between px-1">
                <span className="font-mono text-sm font-semibold">#{tag.name}</span>
                <div className="flex items-center gap-2">
                  <span className="text-2xs text-faint font-mono">{tagTasks.length}</span>
                  <button
                    onClick={() => {
                      if (confirm(`#${tag.name} を削除しますか？`)) deleteTag(tag.id);
                    }}
                    className="text-faint hover:text-high"
                  >
                    ×
                  </button>
                </div>
              </div>
              <div className="flex-1 overflow-y-auto flex flex-col gap-2 pr-1" style={{ scrollbarWidth: 'thin' }}>
                {tagTasks.length === 0 ? (
                  <div className="text-2xs text-faint font-mono text-center py-4">タスクなし</div>
                ) : (
                  tagTasks.map((t) => (
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
