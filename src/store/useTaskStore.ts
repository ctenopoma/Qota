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
  startTimer: (id: string) => Promise<void>;
  stopTimer: (id: string) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;
  duplicateTask: (id: string) => Promise<void>;

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
    try {
      const updatedTasks = await api.updateTask(id, patch);
      set((s) => {
        const primaryTask = updatedTasks.find(x => x.id === id) || s.tasks.find(x => x.id === id);
        const rid = primaryTask?.recurrenceId;
        
        let newTasks = s.tasks;
        if (rid && updatedTasks.length > 1) {
          const currentDue = primaryTask?.due || '';
          // Remove old future uncompleted instances (zombie cleanup) matching the backend
          newTasks = newTasks.filter(t => {
            if (t.id === id) return true; // keep the task being updated
            if (t.recurrenceId === rid && t.status === 'todo') {
              const xDue = t.due || '';
              if (xDue > currentDue) return false; // remove strictly future instances
            }
            return true;
          });
        }

        return {
          tasks: newTasks.map((t) => {
            const u = updatedTasks.find(x => x.id === t.id);
            return u ? u : t;
          }).concat(updatedTasks.filter(u => !newTasks.some(t => t.id === u.id)))
        };
      });
      if (patch.interval !== undefined || patch.recurrenceEnd !== undefined) {
        get().load();
      }
    } catch (e: any) {
      console.error("updateTask error:", e);
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

  startTimer: async (id) => {
    try {
      const now = new Date().toISOString();
      await get().updateTask(id, { startedAt: now });
    } catch (e: any) {
      console.error("startTimer error:", e);
    }
  },

  stopTimer: async (id) => {
    try {
      const task = get().tasks.find((t) => t.id === id);
      if (!task?.startedAt) return;
      const elapsed = Math.round((Date.now() - new Date(task.startedAt).getTime()) / 60000);
      const newActual = (task.actualMinutes ?? 0) + Math.max(elapsed, 1);
      await get().updateTask(id, { actualMinutes: newActual, startedAt: null });
    } catch (e: any) {
      console.error("stopTimer error:", e);
    }
  },

  deleteTask: async (id) => {
    await api.deleteTask(id);
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) }));
  },

  duplicateTask: async (id) => {
    const s = get();
    const t = s.tasks.find((x) => x.id === id);
    if (!t) return;
    
    const nt = await s.addTask({
      title: t.title + ' (コピー)',
      notes: t.notes,
      priority: t.priority,
      due: t.due,
      projectId: t.projectId,
      tagIds: t.tagIds,
      interval: t.interval,
      recurrenceEnd: t.recurrenceEnd,
      recurrenceId: t.recurrenceId,
    });
    
    for (const sub of t.subtasks) {
      await s.addSubtask(nt.id, sub.title);
    }
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
