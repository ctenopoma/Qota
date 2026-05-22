// 日付ヘルパー。すべて ISO 8601 文字列 (YYYY-MM-DD) を入出力する。

import type { Task } from '@/types';

export const sortByDue = (tasks: Task[]): Task[] => {
  return [...tasks].sort((a, b) => {
    if (!a.due && !b.due) return 0;
    if (!a.due) return 1;
    if (!b.due) return -1;
    return a.due.localeCompare(b.due);
  });
};

export const todayISO = (): string => {
  const d = new Date();
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const date = String(d.getDate()).padStart(2, '0');
  return `${year}-${month}-${date}`;
};

export const isToday = (iso: string | null): boolean => iso === todayISO();

export const isTomorrow = (iso: string | null): boolean => {
  if (!iso) return false;
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const tomorrowISO = `${tomorrow.getFullYear()}-${String(tomorrow.getMonth() + 1).padStart(2, '0')}-${String(tomorrow.getDate()).padStart(2, '0')}`;
  return iso === tomorrowISO;
};

export const isOverdue = (iso: string | null): boolean => {
  if (!iso) return false;
  return iso < todayISO();
};

export const isThisWeek = (iso: string | null, weekStart: 'mon' | 'sun' = 'mon'): boolean => {
  if (!iso) return false;
  const d = new Date(iso + 'T00:00:00');
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dow = today.getDay(); // 0=Sun
  const offset = weekStart === 'mon' ? (dow === 0 ? 6 : dow - 1) : dow;
  const start = new Date(today);
  start.setDate(today.getDate() - offset);
  const end = new Date(start);
  end.setDate(start.getDate() + 7);
  return d >= start && d < end;
};

export const isThisMonth = (iso: string | null): boolean => {
  if (!iso) return false;
  const d = new Date(iso + 'T00:00:00');
  const today = new Date();
  return d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth();
};

export const isNextMonth = (iso: string | null): boolean => {
  if (!iso) return false;
  const d = new Date(iso + 'T00:00:00');
  const today = new Date();
  const nextMonthYear = today.getMonth() === 11 ? today.getFullYear() + 1 : today.getFullYear();
  const nextMonthMonth = today.getMonth() === 11 ? 0 : today.getMonth() + 1;
  return d.getFullYear() === nextMonthYear && d.getMonth() === nextMonthMonth;
};

export const formatDue = (iso: string | null): string => {
  if (!iso) return '';
  if (isToday(iso)) return '今日';
  if (isTomorrow(iso)) return '明日';
  const d = new Date(iso + 'T00:00:00');
  return `${d.getMonth() + 1}/${d.getDate()}`;
};
