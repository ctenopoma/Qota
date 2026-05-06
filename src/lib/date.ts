// 日付ヘルパー。すべて ISO 8601 文字列 (YYYY-MM-DD) を入出力する。

export const todayISO = (): string => {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.toISOString().slice(0, 10);
};

export const isToday = (iso: string | null): boolean => iso === todayISO();

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

export const formatDue = (iso: string | null): string => {
  if (!iso) return '';
  const d = new Date(iso + 'T00:00:00');
  const t = todayISO();
  if (iso === t) return '今日';
  // tomorrow
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (iso === tomorrow.toISOString().slice(0, 10)) return '明日';
  return `${d.getMonth() + 1}/${d.getDate()}`;
};
