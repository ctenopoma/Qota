import { useTaskStore } from '@/store/useTaskStore';
import { isOverdue, isToday } from '@/lib/date';

interface Props {
  onClose: () => void;
  onOpen: (id: string) => void;
}

export default function NotifPanel({ onClose, onOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const overdue = tasks.filter((t) => isOverdue(t.due) && t.status !== 'done');
  const today = tasks.filter((t) => isToday(t.due) && t.status !== 'done');

  const Section = ({ title, items, color }: { title: string; items: typeof tasks; color?: string }) => (
    <div>
      <div className="px-4 py-1.5 text-2xs font-mono uppercase tracking-widest text-faint border-b border-border">
        {title} ({items.length})
      </div>
      {items.length === 0 ? (
        <div className="px-4 py-3 text-2xs text-faint">なし</div>
      ) : (
        items.map((t) => (
          <button
            key={t.id}
            onClick={() => onOpen(t.id)}
            className="block w-full px-4 py-2 text-left text-sm hover:bg-panel2 border-b border-border"
            style={color ? { color } : undefined}
          >
            {t.title}
          </button>
        ))
      )}
    </div>
  );

  return (
    <div className="fixed top-3 right-3 bg-panel border border-border rounded-lg w-80 shadow-2xl z-40">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <span className="text-2xs font-mono uppercase tracking-widest">通知</span>
        <button onClick={onClose} className="text-2xs text-faint hover:text-ink">
          ×
        </button>
      </div>
      <Section title="期限切れ" items={overdue} color="oklch(var(--c-high))" />
      <Section title="今日" items={today} />
    </div>
  );
}
