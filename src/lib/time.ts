import type { Task } from '@/types';

export function getEffectiveEstimatedMinutes(task: Task): number | null {
  const hasSubtaskEstimates = task.subtasks.some(s => s.estimatedMinutes != null);
  if (hasSubtaskEstimates) {
    const sum = task.subtasks.reduce((acc, s) => acc + (s.estimatedMinutes ?? 0), 0);
    return sum || null;
  }
  return task.estimatedMinutes ?? null;
}

export function getEffectiveActualMinutes(task: Task): number | null {
  const hasSubtaskActuals = task.subtasks.some(s => s.actualMinutes != null);
  if (hasSubtaskActuals) {
    const sum = task.subtasks.reduce((acc, s) => acc + (s.actualMinutes ?? 0), 0);
    return sum || null;
  }
  return task.actualMinutes ?? null;
}

export function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}分`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m > 0 ? `${h}h ${m}分` : `${h}h`;
}
