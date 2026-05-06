import { useTaskStore } from '@/store/useTaskStore';
import TaskTable from '@/components/TaskTable';

interface Props {
  id: string;
  onOpen: (id: string) => void;
}

export default function ProjectDetail({ id, onOpen }: Props) {
  const projects = useTaskStore((s) => s.projects);
  const tasks = useTaskStore((s) => s.tasks);
  const project = projects.find((p) => p.id === id);
  if (!project) {
    return <div className="p-6 text-mute text-sm">プロジェクトが見つかりません</div>;
  }
  const visible = tasks.filter((t) => t.projectId === id);
  return (
    <TaskTable
      title={project.name}
      subtitle={`${visible.filter((t) => t.status !== 'done').length} todo`}
      tasks={visible}
      project={project}
      onOpen={onOpen}
    />
  );
}
