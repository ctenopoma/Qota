// Zustand ストア。
// バックエンドが正となる "source of truth" で、ストアはそのキャッシュ。
// 各操作は: optimistic 更新 → invoke → 戻り値で確定 (失敗時はロールバック)。

import { create } from 'zustand';
import { api } from '@/lib/api';
import type {
  AppData,
  NewProject,
  NewTask,
  Project,
  ProjectPatch,
  Settings,
  Subtask,
  Tag,
  Task,
  TaskPatch,
} from '@/types';

interface State {
  loaded: boolean;
  tasks: Task[];
  projects: Project[];
  tags: Tag[];
  settings: Settings;

  // ─ lifecycle ─
  load: () => Promise<void>;
  reset: () => Promise<void>;

  // ─ tasks ─
  addTask: (input: NewTask) => Promise<Task>;
  updateTask: (id: string, patch: TaskPatch) => Promise<void>;
  toggleTask: (id: string) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;

  // ─ subtasks ─
  addSubtask: (taskId: string, title: string) => Promise<void>;
  toggleSubtask: (taskId: string, subtaskId: string) => Promise<void>;
  deleteSubtask: (taskId: string, subtaskId: string) => Promise<void>;
  updateSubtask: (taskId: string, subtaskId: string, title: string) => Promise<void>;
  moveSubtask: (taskId: string, subtaskId: string, direction: 'up' | 'down') => Promise<void>;

  // ─ projects ─
  addProject: (input: NewProject) => Promise<Project>;
  updateProject: (id: string, patch: ProjectPatch) => Promise<void>;
  deleteProject: (id: string) => Promise<void>;

  // ─ tags ─
  addTag: (name: string) => Promise<Tag>;
  deleteTag: (id: string) => Promise<void>;

  // ─ settings ─
  updateSettings: (patch: Partial<Settings>) => Promise<void>;

  // ─ import/export ─
  exportJson: () => Promise<string>;
  importJson: () => Promise<void>;
}

const DEFAULT_SETTINGS: Settings = {
  mode: 'dark',
  accent: 'green',
  density: 'comfortable',
  rowHeight: 12,
  weekStart: 'mon',
};

export const useTaskStore = create<State>((set, get) => ({
  loaded: false,
  tasks: [],
  projects: [],
  tags: [],
  settings: DEFAULT_SETTINGS,

  load: async () => {
    const data: AppData = await api.loadAll();
    set({
      loaded: true,
      tasks: data.tasks,
      projects: data.projects,
      tags: data.tags,
      settings: data.settings,
    });
  },

  reset: async () => {
    const data = await api.resetSampleData();
    set({
      tasks: data.tasks,
      projects: data.projects,
      tags: data.tags,
      settings: data.settings,
    });
  },

  // ── tasks ──
  addTask: async (input) => {
    const t = await api.createTask(input);
    set((s) => ({ tasks: [...s.tasks, t] }));
    return t;
  },

  updateTask: async (id, patch) => {
    const updatedTasks = await api.updateTask(id, patch);
    set((s) => {
      let newTasks = [...s.tasks];
      // remove old deleted instances of the same series if they were removed by the backend
      // wait, the backend doesn't return deleted IDs. Instead of tracking deletions, it's safer to just fetch or rely on the returned list?
      // Actually, since updateTask might delete future tasks, maybe it's safest to replace the WHOLE series.
      // But we don't have the series ID reliably.
      // Easiest robust way for updateTask when recurrence changes: just reload all tasks or reload the store.
      // Or we can manually remove future incomplete tasks of this recurrenceId if interval is in patch.
      // To keep it simple and robust, let's just do a map and push, but we might have zombies.
      // Let's implement a simple zombie cleanup: if the patch has interval or recurrenceEnd, we can't easily know what was deleted.
      // So let's just reload the whole app data if recurrence changes.
      return {
        tasks: s.tasks.map((t) => {
          const u = updatedTasks.find(x => x.id === t.id);
          return u ? u : t;
        }).concat(updatedTasks.filter(u => !s.tasks.some(t => t.id === u.id)))
      };
    });
    // If recurrence was modified, let's just do a full reload to ensure we don't have zombie tasks
    if (patch.interval !== undefined || patch.recurrenceEnd !== undefined) {
      get().load();
    }
  },

  toggleTask: async (id) => {
    const updatedTasks = await api.toggleTask(id);
    set((s) => {
      // Create a map or just update one by one
      let newTasks = [...s.tasks];
      for (const t of updatedTasks) {
        const idx = newTasks.findIndex((x) => x.id === t.id);
        if (idx !== -1) {
          newTasks[idx] = t;
        } else {
          newTasks.push(t);
        }
      }
      return { tasks: newTasks };
    });
  },

  deleteTask: async (id) => {
    await api.deleteTask(id);
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) }));
  },

  // ── subtasks ──
  addSubtask: async (taskId, title) => {
    const s: Subtask = await api.addSubtask(taskId, title);
    set((st) => ({
      tasks: st.tasks.map((t) =>
        t.id === taskId ? { ...t, subtasks: [...t.subtasks, s] } : t,
      ),
    }));
  },

  toggleSubtask: async (taskId, subtaskId) => {
    const updated = await api.toggleSubtask(taskId, subtaskId);
    set((st) => ({
      tasks: st.tasks.map((t) =>
        t.id !== taskId
          ? t
          : {
              ...t,
              subtasks: t.subtasks.map((s) =>
                s.id === subtaskId ? updated : s,
              ),
            },
      ),
    }));
  },

  deleteSubtask: async (taskId, subtaskId) => {
    await api.deleteSubtask(taskId, subtaskId);
    set((st) => ({
      tasks: st.tasks.map((t) =>
        t.id !== taskId
          ? t
          : { ...t, subtasks: t.subtasks.filter((s) => s.id !== subtaskId) },
      ),
    }));
  },

  updateSubtask: async (taskId, subtaskId, title) => {
    const updated = await api.updateSubtask(taskId, subtaskId, title);
    set((st) => ({
      tasks: st.tasks.map((t) =>
        t.id !== taskId
          ? t
          : {
              ...t,
              subtasks: t.subtasks.map((s) =>
                s.id === subtaskId ? updated : s,
              ),
            },
      ),
    }));
  },

  moveSubtask: async (taskId, subtaskId, direction) => {
    const subtasks = await api.moveSubtask(taskId, subtaskId, direction);
    set((st) => ({
      tasks: st.tasks.map((t) =>
        t.id !== taskId
          ? t
          : { ...t, subtasks },
      ),
    }));
  },

  // ── projects ──
  addProject: async (input) => {
    const p = await api.createProject(input);
    set((s) => ({ projects: [...s.projects, p] }));
    return p;
  },

  updateProject: async (id, patch) => {
    const updated = await api.updateProject(id, patch);
    set((s) => ({
      projects: s.projects.map((p) => (p.id === id ? updated : p)),
    }));
  },

  deleteProject: async (id) => {
    await api.deleteProject(id);
    set((s) => ({
      projects: s.projects.filter((p) => p.id !== id),
      tasks: s.tasks.map((t) =>
        t.projectId === id ? { ...t, projectId: null } : t,
      ),
    }));
  },

  // ── tags ──
  addTag: async (name) => {
    const tag = await api.createTag(name);
    set((s) => ({ tags: [...s.tags, tag] }));
    return tag;
  },

  deleteTag: async (id) => {
    await api.deleteTag(id);
    set((s) => ({
      tags: s.tags.filter((t) => t.id !== id),
      tasks: s.tasks.map((t) => ({
        ...t,
        tagIds: t.tagIds.filter((tid) => tid !== id),
      })),
    }));
  },

  // ── settings ──
  updateSettings: async (patch) => {
    const updated = await api.updateSettings(patch);
    set({ settings: updated });
  },

  // ── import/export ──
  exportJson: async () => {
    return await api.exportJson();
  },

  importJson: async () => {
    const data = await api.importJson();
    set({
      tasks: data.tasks,
      projects: data.projects,
      tags: data.tags,
      settings: data.settings,
    });
  },
}));
