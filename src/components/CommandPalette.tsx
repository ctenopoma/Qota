import { useEffect, useRef, useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import type { Route } from '@/types';

interface Props {
  onClose: () => void;
  onRoute: (r: Route) => void;
}

interface Item {
  label: string;
  sub: string;
  action: () => void;
}

export default function CommandPalette({ onClose, onRoute }: Props) {
  const [q, setQ] = useState('');
  const [idx, setIdx] = useState(0);
  const projects = useTaskStore((s) => s.projects);
  const inputRef = useRef<HTMLInputElement>(null);

  const items: Item[] = [
    { label: '→ 今日', sub: 'T', action: () => onRoute({ view: 'today' }) },
    { label: '→ 受信箱', sub: 'I', action: () => onRoute({ view: 'inbox' }) },
    { label: '→ 今後', sub: 'U', action: () => onRoute({ view: 'upcoming' }) },
    { label: '→ プロジェクト一覧', sub: 'P', action: () => onRoute({ view: 'projects' }) },
    { label: '→ タグ', sub: '#', action: () => onRoute({ view: 'tags' }) },
    { label: '→ 設定', sub: ',', action: () => onRoute({ view: 'settings' }) },
    ...projects.map((p) => ({
      label: `▸ ${p.name}`,
      sub: 'PROJECT',
      action: () => onRoute({ view: 'project', id: p.id }),
    })),
  ];

  const filtered = q
    ? items.filter((it) => it.label.toLowerCase().includes(q.toLowerCase()))
    : items;

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setIdx(0);
  }, [q]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setIdx((i) => Math.min(filtered.length - 1, i + 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setIdx((i) => Math.max(0, i - 1));
    } else if (e.key === 'Enter') {
      filtered[idx]?.action();
      onClose();
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 grid place-items-start pt-[20vh] z-50" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="bg-panel border border-border rounded-lg w-[480px] shadow-2xl"
      >
        <input
          ref={inputRef}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={onKey}
          placeholder="コマンドを検索…"
          className="w-full bg-transparent px-4 py-3 outline-none border-b border-border text-sm"
        />
        <div className="max-h-80 overflow-y-auto py-1">
          {filtered.map((it, i) => (
            <button
              key={it.label}
              onClick={() => {
                it.action();
                onClose();
              }}
              onMouseEnter={() => setIdx(i)}
              className={`flex justify-between items-center w-full px-4 py-1.5 text-left text-sm ${
                i === idx ? 'bg-panel2' : ''
              }`}
            >
              <span>{it.label}</span>
              <kbd className="font-mono text-2xs text-faint">{it.sub}</kbd>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
