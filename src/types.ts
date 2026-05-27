// 全データの型定義。バックエンドの JSON スキーマと 1:1 対応。

export type Priority = 'high' | 'medium' | 'low' | null;
export type TaskStatus = 'todo' | 'done';

export interface Subtask {
  id: string;
  title: string;
  done: boolean;
  estimatedMinutes?: number | null;
  actualMinutes?: number | null;
  startedAt?: string | null;
}

export type SubtaskPatch = Partial<Pick<Subtask, 'title' | 'estimatedMinutes' | 'actualMinutes' | 'startedAt'>>;

export interface Task {
  id: string;
  title: string;
  notes: string;
  status: TaskStatus;
  priority: Priority;
  /** ISO 8601 date string (YYYY-MM-DD) or null */
  due: string | null;
  projectId: string | null;
  tagIds: string[];
  subtasks: Subtask[];
  /** ISO 8601 datetime */
  createdAt: string;
  updatedAt: string;
  /** ISO 8601 datetime, null if not done */
  completedAt: string | null;
  estimatedMinutes?: number | null;
  actualMinutes?: number | null;
  startedAt?: string | null;
}

export interface Project {
  id: string;
  name: string;
  color: string; // oklch / hex 等
  archived: boolean;
  createdAt: string;
}

export interface Tag {
  id: string;
  name: string;
  createdAt: string;
}

export interface Settings {
  mode: 'dark' | 'light';
  accent: 'green' | 'blue' | 'amber' | 'violet';
  density: 'compact' | 'comfortable' | 'spacious';
  rowHeight: number;
  weekStart: 'mon' | 'sun';
}

export interface AppData {
  schemaVersion: number;
  tasks: Task[];
  projects: Project[];
  tags: Tag[];
  settings: Settings;
}

export type Interval = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'weekdays';

export interface Task {
  id: string;
  title: string;
  notes: string;
  status: TaskStatus;
  priority: Priority;
  due: string | null;
  projectId: string | null;
  tagIds: string[];
  subtasks: Subtask[];
  interval?: Interval | null;
  recurrenceId?: string | null;
  recurrenceEnd?: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  estimatedMinutes?: number | null;
  actualMinutes?: number | null;
  startedAt?: string | null;
}

// 入力 DTO
export type NewTask = Pick<Task, 'title'> &
  Partial<Pick<Task, 'notes' | 'priority' | 'due' | 'projectId' | 'tagIds' | 'interval' | 'recurrenceId' | 'recurrenceEnd' | 'estimatedMinutes'>>;

export type TaskPatch = Partial<
  Pick<
    Task,
    | 'title'
    | 'notes'
    | 'status'
    | 'priority'
    | 'due'
    | 'projectId'
    | 'tagIds'
    | 'interval'
    | 'recurrenceEnd'
    | 'estimatedMinutes'
    | 'actualMinutes'
    | 'startedAt'
  >
>;

export type NewProject = Pick<Project, 'name'> & Partial<Pick<Project, 'color'>>;
export type ProjectPatch = Partial<Pick<Project, 'name' | 'color' | 'archived'>>;

export type Route =
  | { view: 'today' }
  | { view: 'inbox' }
  | { view: 'upcoming' }
  | { view: 'calendar' }
  | { view: 'projects' }
  | { view: 'project'; id: string }
  | { view: 'tags' }
  | { view: 'settings' }
  | { view: 'insights' };
