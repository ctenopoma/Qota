// Tauri invoke の薄いラッパー。
// バックエンド側 (docs/BACKEND.md) で定義する commands を 1:1 で呼ぶ。

import { invoke } from '@tauri-apps/api/core';
import type {
  AppData,
  NewProject,
  NewTask,
  Project,
  ProjectPatch,
  Settings,
  Subtask,
  SubtaskPatch,
  Tag,
  Task,
  TaskPatch,
} from '@/types';

export const api = {
  // ───── snapshot ─────
  loadAll: () => invoke<AppData>('load_all'),

  // ───── tasks ─────
  createTask: (input: NewTask) => invoke<Task>('create_task', { input }),
  updateTask: (id: string, patch: TaskPatch) =>
    invoke<Task[]>('update_task', { id, patch }),
  deleteTask: (id: string) => invoke<void>('delete_task', { id }),
  toggleTask: (id: string) =>
    invoke<Task[]>('toggle_task', { id }),

  // ───── subtasks ─────
  addSubtask: (taskId: string, title: string) =>
    invoke<Subtask>('add_subtask', { taskId, title }),
  toggleSubtask: (taskId: string, subtaskId: string) =>
    invoke<Subtask>('toggle_subtask', { taskId, subtaskId }),
  deleteSubtask: (taskId: string, subtaskId: string) =>
    invoke<void>('delete_subtask', { taskId, subtaskId }),
  updateSubtask: (taskId: string, subtaskId: string, patch: SubtaskPatch) =>
    invoke<Subtask>('update_subtask', { taskId, subtaskId, patch }),
  moveSubtask: (taskId: string, subtaskId: string, direction: 'up' | 'down') =>
    invoke<Subtask[]>('move_subtask', { taskId, subtaskId, direction }),
  startSubtaskTimer: (taskId: string, subtaskId: string) =>
    invoke<Subtask>('start_subtask_timer', { taskId, subtaskId }),
  stopSubtaskTimer: (taskId: string, subtaskId: string) =>
    invoke<Subtask>('stop_subtask_timer', { taskId, subtaskId }),

  // ───── projects ─────
  createProject: (input: NewProject) =>
    invoke<Project>('create_project', { input }),
  updateProject: (id: string, patch: ProjectPatch) =>
    invoke<Project>('update_project', { id, patch }),
  deleteProject: (id: string) => invoke<void>('delete_project', { id }),

  // ───── tags ─────
  createTag: (name: string) => invoke<Tag>('create_tag', { name }),
  deleteTag: (id: string) => invoke<void>('delete_tag', { id }),

  // ───── settings ─────
  updateSettings: (patch: Partial<Settings>) =>
    invoke<Settings>('update_settings', { patch }),

  // ───── search ─────
  search: (q: string) => invoke<Task[]>('search', { q }),

  // ───── import / export ─────
  exportJson: () => invoke<string>('export_json'), // 保存パスを返す
  importJson: () => invoke<AppData>('import_json'), // 読み込んだ AppData を返す
  resetSampleData: () => invoke<AppData>('reset_sample_data'),
};
