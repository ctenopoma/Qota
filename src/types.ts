// 全データの型定義。バックエンドの JSON スキーマと 1:1 対応。

export type Priority = 'high' | 'medium' | 'low' | null;
export type TaskStatus = 'todo' | 'done';

export interface Subtask {
  id: string;
  title: string;
  done: boolean;
}

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

/** バックエンドが返す丸ごとのスナップショット */
export interface AppData {
  schemaVersion: number;
  tasks: Task[];
  projects: Project[];
  tags: Tag[];
  settings: Settings;
}

// 入力 DTO
export type NewTask = Pick<Task, 'title'> &
  Partial<Pick<Task, 'notes' | 'priority' | 'due' | 'projectId' | 'tagIds'>>;

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
  >
>;

export type NewProject = Pick<Project, 'name'> & Partial<Pick<Project, 'color'>>;
export type ProjectPatch = Partial<Pick<Project, 'name' | 'color' | 'archived'>>;

// View ルーティング
export type Route =
  | { view: 'today' }
  | { view: 'inbox' }
  | { view: 'upcoming' }
  | { view: 'projects' }
  | { view: 'project'; id: string }
  | { view: 'tags' }
  | { view: 'settings' };
