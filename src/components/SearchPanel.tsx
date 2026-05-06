import { useEffect, useRef, useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { formatDue } from '@/lib/date';

interface Props {
  onClose: () => void;
  onOpen: (id: string) => void;
}

export default function SearchPanel({ onClose, onOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const [q, setQ] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // クライアント側の素朴な部分一致。本実装はバックエンドの search コマンドを呼んでも良い。
  const results = q
    ? tasks.filter(
        (t) =>
          t.title.toLowerCase().includes(q.toLowerCase()) ||
          (t.notes ?? '').toLowerCase().includes(q.toLowerCase()),
      )
    : [];

  return (
    <div className="fixed inset-0 bg-black/40 grid place-items-start pt-[15vh] z-50" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="bg-panel border border-border rounded-lg w-[560px] shadow-2xl"
      >
        <input
          ref={inputRef}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="タスクを全文検索…"
          className="w-full bg-transparent px-4 py-3 outline-none border-b border-border text-sm"
        />
        <div className="max-h-96 overflow-y-auto">
          {!q && (
            <div className="px-4 py-8 text-center text-2xs text-faint font-mono">
              検索キーワードを入力
            </div>
          )}
          {q && results.length === 0 && (
            <div className="px-4 py-8 text-center text-2xs text-faint font-mono">
              一致するタスクなし
            </div>
          )}
          {results.map((t) => (
            <button
              key={t.id}
              onClick={() => onOpen(t.id)}
              className="flex justify-between items-center w-full px-4 py-2 text-left text-sm hover:bg-panel2 border-b border-border last:border-b-0"
            >
              <span className={t.status === 'done' ? 'line-through text-mute' : ''}>{t.title}</span>
              <span className="text-2xs font-mono text-faint">{formatDue(t.due)}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
