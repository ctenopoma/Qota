import { useTaskStore } from '@/store/useTaskStore';
import { isOverdue, isToday } from '@/lib/date';
import TaskTable from '@/components/TaskTable';

export default function Today({ onOpen }: { onOpen: (id: string) => void }) {
  const tasks = useTaskStore((s) => s.tasks);
  const visible = tasks.filter(
    (t) => isToday(t.due) || (isOverdue(t.due) && t.status !== 'done'),
  );
  return <TaskTable title="今日" tasks={visible} project={null} onOpen={onOpen} />;
}
