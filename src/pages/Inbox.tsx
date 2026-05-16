import { useTaskStore } from '@/store/useTaskStore';
import { sortByDue } from '@/lib/date';
import TaskTable from '@/components/TaskTable';

export default function Inbox({ onOpen }: { onOpen: (id: string) => void }) {
  const tasks = useTaskStore((s) => s.tasks);
  const visible = sortByDue(tasks.filter((t) => !t.projectId));
  return (
    <TaskTable
      title="受信箱"
      subtitle="プロジェクト未割当"
      tasks={visible}
      project={null}
      onOpen={onOpen}
    />
  );
}
