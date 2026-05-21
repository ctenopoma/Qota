import { useState } from 'react';
import type { Route } from '@/types';
import { useTaskStore } from '@/store/useTaskStore';
import { isOverdue } from '@/lib/date';

interface Props {
  route: Route;
  onRoute: (r: Route) => void;
  onCmd: () => void;
  onSearch: () => void;
  onNotif: () => void;
  onToggleSidebar: () => void;
  isOpen: boolean;
}

export default function Sidebar({ route, onRoute, onCmd, onSearch, onNotif, onToggleSidebar, isOpen }: Props) {
  const tasks = useTaskStore((s) => s.tasks);
  const projects = useTaskStore((s) => s.projects);
  const addProject = useTaskStore((s) => s.addProject);
  const overdueCount = tasks.filter((t) => isOverdue(t.due) && t.status !== 'done').length;

  const [isAddingProject, setIsAddingProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState('');

  const submitProject = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newProjectName.trim()) {
      setIsAddingProject(false);
      return;
    }
    await addProject({ name: newProjectName.trim() });
    setNewProjectName('');
    setIsAddingProject(false);
  };

  const isActive = (test: (r: Route) => boolean) => test(route);
  const navBtn = (active: boolean) =>
    `flex items-center gap-2 w-full px-2 py-1 rounded text-2xs font-mono uppercase tracking-wide ${
      active ? 'bg-panel2 text-ink' : 'text-mute hover:text-ink'
    }`;

  return (
    <aside className={`border-r border-border bg-panel flex flex-col p-3 gap-3 text-sm overflow-y-auto transition-all ${!isOpen ? 'items-center' : ''}`}>
      {isOpen && (
        <>
          <div className="flex items-center justify-between">
            <span className="font-mono text-2xs text-faint tracking-widest">QOTA</span>
            <div className="flex gap-1">
              <button
                onClick={onSearch}
                className="px-1.5 py-0.5 rounded text-2xs text-mute hover:text-ink border border-border"
                title="検索 (⌘/)"
              >
                /
              </button>
              <button
                onClick={onNotif}
                className="px-1.5 py-0.5 rounded text-2xs text-mute hover:text-ink border border-border relative"
                title="通知"
              >
                !
                {overdueCount > 0 && (
                  <span className="absolute -top-1 -right-1 bg-high text-bg text-2xs leading-none rounded-full w-3 h-3 grid place-items-center">
                    {overdueCount}
                  </span>
                )}
              </button>
            </div>
          </div>

          <button
            onClick={onCmd}
            className="flex items-center justify-between border border-border rounded px-2 py-1.5 text-2xs text-mute hover:text-ink"
          >
            <span>コマンド…</span>
            <kbd className="font-mono text-2xs">⌘K</kbd>
          </button>

          <nav className="flex flex-col gap-0.5">
            <button onClick={() => onRoute({ view: 'today' })} className={navBtn(isActive((r) => r.view === 'today'))}>
              今日
            </button>
            <button onClick={() => onRoute({ view: 'inbox' })} className={navBtn(isActive((r) => r.view === 'inbox'))}>
              受信箱
            </button>
            <button onClick={() => onRoute({ view: 'upcoming' })} className={navBtn(isActive((r) => r.view === 'upcoming'))}>
              今後
            </button>
          </nav>

          <div className="mt-2">
            <div className="flex items-center justify-between mb-1">
              <span className="font-mono text-2xs text-faint tracking-widest">PROJECTS</span>
              <div className="flex gap-2">
                <button
                  onClick={() => setIsAddingProject(true)}
                  className="text-2xs text-mute hover:text-ink"
                  title="プロジェクトを追加"
                >
                  ＋
                </button>
                <button
                  onClick={() => onRoute({ view: 'projects' })}
                  className="text-2xs text-mute hover:text-ink"
                >
                  一覧
                </button>
              </div>
            </div>
            {isAddingProject && (
              <form onSubmit={submitProject} className="mb-2">
                <input
                  autoFocus
                  value={newProjectName}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  onBlur={() => {
                    if (!newProjectName.trim()) setIsAddingProject(false);
                  }}
                  placeholder="プロジェクト名..."
                  className="w-full bg-bg border border-border rounded px-2 py-1 text-2xs outline-none focus:border-high placeholder:text-faint"
                />
              </form>
            )}
            <div className="flex flex-col gap-0.5">
              {projects.filter((p) => !p.archived).map((p) => (
                <button
                  key={p.id}
                  onClick={() => onRoute({ view: 'project', id: p.id })}
                  className={navBtn(isActive((r) => r.view === 'project' && r.id === p.id))}
                >
                  <span className="w-1.5 h-1.5 rounded-full" style={{ background: p.color }} />
                  <span className="truncate">{p.name}</span>
                </button>
              ))}
            </div>
          </div>

          <nav className="flex flex-col gap-0.5 mt-auto">
            <button onClick={() => onRoute({ view: 'tags' })} className={navBtn(isActive((r) => r.view === 'tags'))}>
              タグ
            </button>
            <button onClick={() => onRoute({ view: 'settings' })} className={navBtn(isActive((r) => r.view === 'settings'))}>
              設定
            </button>
          </nav>
        </>
      )}

      {!isOpen && (
        <nav className={`flex ${!isOpen ? 'flex-col' : 'gap-0.5'} gap-1`}>
          <button
            onClick={onSearch}
            className="px-1.5 py-0.5 rounded text-2xs text-mute hover:text-ink border border-border"
            title="検索 (⌘/)"
          >
            /
          </button>
          <button
            onClick={onNotif}
            className="px-1.5 py-0.5 rounded text-2xs text-mute hover:text-ink border border-border relative"
            title="通知"
          >
            !
            {overdueCount > 0 && (
              <span className="absolute -top-1 -right-1 bg-high text-bg text-2xs leading-none rounded-full w-3 h-3 grid place-items-center">
                {overdueCount}
              </span>
            )}
          </button>
        </nav>
      )}

      <button
        onClick={onToggleSidebar}
        className="px-1.5 py-0.5 rounded text-2xs text-mute hover:text-ink border border-border mt-auto"
        title={isOpen ? 'メニューを閉じる' : 'メニューを開く'}
      >
        {isOpen ? '◀' : '▶'}
      </button>
    </aside>
  );
}
