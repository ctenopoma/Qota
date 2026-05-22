import { useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import type { Task } from '@/types';

interface Props {
  onOpen: (id: string) => void;
}

const getDaysInMonth = (year: number, month: number) => new Date(year, month + 1, 0).getDate();
const getFirstDayOfMonth = (year: number, month: number) => new Date(year, month, 1).getDay(); // 0: Sun, 1: Mon...

const formatDayIso = (year: number, month: number, day: number) => {
  return `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
};

export default function Calendar({ onOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const projects = useTaskStore((s) => s.projects);
  const weekStart = useTaskStore((s) => s.settings.weekStart);

  const [currentDate, setCurrentDate] = useState(new Date());
  const [selectedDayInfo, setSelectedDayInfo] = useState<{ iso: string; tasks: Task[]; day: number } | null>(null);

  const year = currentDate.getFullYear();
  const month = currentDate.getMonth();

  const prevMonth = () => setCurrentDate(new Date(year, month - 1, 1));
  const nextMonth = () => setCurrentDate(new Date(year, month + 1, 1));
  const goToToday = () => setCurrentDate(new Date());

  const daysInMonth = getDaysInMonth(year, month);
  const firstDay = getFirstDayOfMonth(year, month);

  const offset = weekStart === 'mon' ? (firstDay === 0 ? 6 : firstDay - 1) : firstDay;
  const blanks = Array.from({ length: offset });
  const days = Array.from({ length: daysInMonth }, (_, i) => i + 1);

  const weekDayLabels = weekStart === 'mon' 
    ? ['月', '火', '水', '木', '金', '土', '日']
    : ['日', '月', '火', '水', '木', '金', '土'];

  const getTasksForDay = (day: number) => {
    const iso = formatDayIso(year, month, day);
    return tasks.filter((t) => t.due === iso);
  };

  const isToday = (day: number) => {
    const today = new Date();
    return today.getFullYear() === year && today.getMonth() === month && today.getDate() === day;
  };

  return (
    <div className="flex flex-col h-full bg-bg relative">
      <header className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-base font-semibold">
            {year}年 {month + 1}月
          </h1>
          <div className="flex items-center gap-1">
            <button onClick={prevMonth} className="p-1 rounded hover:bg-panel border border-transparent hover:border-border text-mute">
              ◀
            </button>
            <button onClick={nextMonth} className="p-1 rounded hover:bg-panel border border-transparent hover:border-border text-mute">
              ▶
            </button>
            <button onClick={goToToday} className="px-2 py-1 rounded hover:bg-panel border border-transparent hover:border-border text-2xs font-mono text-mute ml-2">
              今日
            </button>
          </div>
        </div>
      </header>

      <div className="flex-1 flex flex-col p-4 overflow-hidden">
        <div className="grid grid-cols-7 gap-px mb-1">
          {weekDayLabels.map((label) => (
            <div key={label} className="text-center text-2xs font-mono text-faint uppercase py-1">
              {label}
            </div>
          ))}
        </div>
        <div className="flex-1 grid grid-cols-7 auto-rows-[minmax(80px,1fr)] gap-px bg-border border border-border overflow-y-auto">
          {blanks.map((_, i) => (
            <div key={`blank-${i}`} className="bg-bg p-1" />
          ))}
          {days.map((day) => {
            const dayTasks = getTasksForDay(day);
            const todayClass = isToday(day) ? 'bg-accent text-bg w-6 h-6 flex items-center justify-center rounded-full mx-auto' : 'w-6 h-6 flex items-center justify-center mx-auto';
            const iso = formatDayIso(year, month, day);
            
            const maxVisible = 2;
            const visibleTasks = dayTasks.slice(0, maxVisible);
            const hasMore = dayTasks.length > maxVisible;
            
            return (
              <div 
                key={day} 
                className="bg-bg p-1 flex flex-col gap-1 overflow-hidden min-h-[80px] hover:bg-panel/30 cursor-pointer"
                onClick={() => setSelectedDayInfo({ iso, tasks: dayTasks, day })}
              >
                <div className="text-2xs font-mono text-center mb-1">
                  <div className={todayClass}>{day}</div>
                </div>
                <div className="flex-1 overflow-hidden flex flex-col gap-1 pr-1">
                  {visibleTasks.map((t) => {
                    const project = projects.find((p) => p.id === t.projectId);
                    return (
                      <button
                        key={t.id}
                        onClick={(e) => { e.stopPropagation(); onOpen(t.id); }}
                        className={`text-left text-2xs truncate px-1.5 py-0.5 rounded border border-border hover:border-mute ${
                          t.status === 'done' ? 'line-through text-mute bg-panel opacity-50' : 'bg-panel'
                        }`}
                        title={t.title}
                      >
                        {project && (
                          <span 
                            className="inline-block w-1.5 h-1.5 rounded-full mr-1 align-middle" 
                            style={{ background: project.color }} 
                          />
                        )}
                        <span className="align-middle">{t.title}</span>
                      </button>
                    );
                  })}
                  {hasMore && (
                    <div className="text-2xs text-mute text-center mt-auto mb-1 hover:text-accent font-semibold">
                      + {dayTasks.length - maxVisible} 件
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {selectedDayInfo && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-bg/50 backdrop-blur-sm p-4" onClick={() => setSelectedDayInfo(null)}>
          <div className="bg-bg border border-border rounded-lg shadow-xl w-full max-w-sm flex flex-col max-h-[80vh] overflow-hidden" onClick={e => e.stopPropagation()}>
            <div className="px-4 py-3 border-b border-border flex items-center justify-between">
              <h2 className="font-semibold">{year}年 {month + 1}月 {selectedDayInfo.day}日</h2>
              <button onClick={() => setSelectedDayInfo(null)} className="text-mute hover:text-fg p-1">
                ✕
              </button>
            </div>
            <div className="p-4 overflow-y-auto flex flex-col gap-2">
              {selectedDayInfo.tasks.length === 0 ? (
                <p className="text-mute text-sm text-center py-4">タスクはありません</p>
              ) : (
                selectedDayInfo.tasks.map(t => {
                  const project = projects.find((p) => p.id === t.projectId);
                  return (
                    <button
                      key={t.id}
                      onClick={() => {
                        setSelectedDayInfo(null);
                        onOpen(t.id);
                      }}
                      className={`text-left text-sm truncate px-3 py-2 rounded border border-border hover:border-mute transition-colors ${
                        t.status === 'done' ? 'line-through text-mute bg-panel opacity-50' : 'bg-panel'
                      }`}
                      title={t.title}
                    >
                      {project && (
                        <span 
                          className="inline-block w-2 h-2 rounded-full mr-2 align-middle" 
                          style={{ background: project.color }} 
                        />
                      )}
                      <span className="align-middle">{t.title}</span>
                    </button>
                  );
                })
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
