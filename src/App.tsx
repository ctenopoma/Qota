import { useEffect, useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import type { Route } from '@/types';
import Sidebar from '@/components/Sidebar';
import CommandPalette from '@/components/CommandPalette';
import SearchPanel from '@/components/SearchPanel';
import NotifPanel from '@/components/NotifPanel';
import TaskDetail from '@/components/TaskDetail';
import Today from '@/pages/Today';
import Inbox from '@/pages/Inbox';
import Upcoming from '@/pages/Upcoming';
import Calendar from '@/pages/Calendar';
import ProjectsList from '@/pages/ProjectsList';
import ProjectDetail from '@/pages/ProjectDetail';
import Tags from '@/pages/Tags';
import Settings from '@/pages/Settings';
import Insights from '@/pages/Insights';

export default function App() {
  const load = useTaskStore((s) => s.load);
  const loaded = useTaskStore((s) => s.loaded);
  const settings = useTaskStore((s) => s.settings);

  const [route, setRoute] = useState<Route>({ view: 'today' });
  const [cmdOpen, setCmdOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [notifOpen, setNotifOpen] = useState(false);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    load().catch((e) => {
      console.error('load failed', e);
      setLoadError(String(e));
    });
  }, [load]);

  // テーマ class を root に反映
  useEffect(() => {
    document.documentElement.classList.toggle('light', settings.mode === 'light');
  }, [settings.mode]);

  // グローバルショートカット
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setCmdOpen(true);
      } else if ((e.metaKey || e.ctrlKey) && e.key === '/') {
        e.preventDefault();
        setSearchOpen(true);
      } else if (e.key === 'Escape') {
        setCmdOpen(false);
        setSearchOpen(false);
        setNotifOpen(false);
        setDetailId(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  if (loadError) {
    return (
      <div className="grid h-full place-items-center text-red-500 text-sm">
        読み込みエラー: {loadError}
      </div>
    );
  }

  if (!loaded) {
    return (
      <div className="grid h-full place-items-center text-mute text-sm">
        読み込み中…
      </div>
    );
  }

  return (
    <div className="grid h-full bg-bg text-ink" style={{ gridTemplateColumns: sidebarOpen ? '220px 1fr' : '60px 1fr' }}>
      <Sidebar
        route={route}
        onRoute={setRoute}
        onCmd={() => setCmdOpen(true)}
        onSearch={() => setSearchOpen(true)}
        onNotif={() => setNotifOpen(true)}
        onToggleSidebar={() => setSidebarOpen(!sidebarOpen)}
        isOpen={sidebarOpen}
      />

      <main className="overflow-hidden">
        {route.view === 'today' && <Today onOpen={setDetailId} />}
        {route.view === 'inbox' && <Inbox onOpen={setDetailId} />}
        {route.view === 'upcoming' && <Upcoming onOpen={setDetailId} />}
        {route.view === 'calendar' && <Calendar onOpen={setDetailId} />}
        {route.view === 'projects' && (
          <ProjectsList onOpen={(id) => setRoute({ view: 'project', id })} />
        )}
        {route.view === 'project' && (
          <ProjectDetail id={route.id} onOpen={setDetailId} />
        )}
        {route.view === 'tags' && <Tags onOpen={setDetailId} />}
        {route.view === 'settings' && <Settings />}
        {route.view === 'insights' && <Insights />}
      </main>

      {cmdOpen && (
        <CommandPalette onClose={() => setCmdOpen(false)} onRoute={setRoute} />
      )}
      {searchOpen && (
        <SearchPanel
          onClose={() => setSearchOpen(false)}
          onOpen={(id) => {
            setSearchOpen(false);
            setDetailId(id);
          }}
        />
      )}
      {notifOpen && (
        <NotifPanel
          onClose={() => setNotifOpen(false)}
          onOpen={(id) => {
            setNotifOpen(false);
            setDetailId(id);
          }}
        />
      )}
      {detailId && (
        <TaskDetail id={detailId} onClose={() => setDetailId(null)} />
      )}
    </div>
  );
}
