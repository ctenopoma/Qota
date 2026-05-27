import { useState, useMemo } from 'react';
import { useTaskStore } from '@/store/useTaskStore';
import { getEffectiveEstimatedMinutes, getEffectiveActualMinutes } from '@/lib/time';
import type { Task } from '@/types';

type Period = 'week' | 'month';

function startOfWeek(d: Date): Date {
  const day = d.getDay();
  const diff = d.getDate() - day + (day === 0 ? -6 : 1);
  const s = new Date(d);
  s.setDate(diff);
  s.setHours(0, 0, 0, 0);
  return s;
}

function startOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), 1);
}

function formatMin(m: number): string {
  if (m < 60) return `${m}分`;
  const h = Math.floor(m / 60);
  const r = m % 60;
  return r > 0 ? `${h}時間${r}分` : `${h}時間`;
}

export default function Insights() {
  const tasks = useTaskStore((s) => s.tasks);
  const [period, setPeriod] = useState<Period>('week');

  const { completedAll, tracked, accuracy, saved, over, messages } = useMemo(() => {
    const now = new Date();
    const cutoff = period === 'week' ? startOfWeek(now) : startOfMonth(now);

    const completedAll = tasks.filter(
      (t) => t.status === 'done' && t.completedAt && new Date(t.completedAt) >= cutoff
    );

    const tracked = completedAll.filter((t) => {
      const est = getEffectiveEstimatedMinutes(t);
      const act = getEffectiveActualMinutes(t);
      return est != null && act != null;
    });

    let accuracy = 0;
    let saved = 0;
    let over = 0;

    if (tracked.length > 0) {
      const totalEst = tracked.reduce((s, t) => s + (getEffectiveEstimatedMinutes(t) ?? 0), 0);
      const totalAct = tracked.reduce((s, t) => s + (getEffectiveActualMinutes(t) ?? 0), 0);
      accuracy = totalEst > 0 ? Math.round((totalAct / totalEst) * 100) : 0;

      for (const t of tracked) {
        const est = getEffectiveEstimatedMinutes(t) ?? 0;
        const act = getEffectiveActualMinutes(t) ?? 0;
        const diff = est - act;
        if (diff > 0) saved += diff;
        else over += Math.abs(diff);
      }
    }

    // Generate coaching messages
    const messages: string[] = [];
    if (tracked.length >= 5) {
      if (accuracy <= 110 && accuracy >= 90) {
        messages.push('見積もり精度が高いです。この調子を維持しましょう！');
      } else if (accuracy > 130) {
        messages.push('見積もりより時間がかかる傾向があります。タスクをもう少し細かく分割してみると改善するかもしれません。');
      } else if (accuracy > 110) {
        messages.push('やや見積もりより時間がかかっています。バッファを少し多めに取ると良さそうです。');
      } else if (accuracy < 70) {
        messages.push('見積もりより大幅に早く完了しています。見積もりを少し短くしても良いかもしれません。');
      } else if (accuracy < 90) {
        messages.push('見積もりより早く完了できています。効率よく作業できていますね！');
      }
      if (saved > over) {
        messages.push(`${period === 'week' ? '今週' : '今月'}は合計 ${formatMin(saved - over)} を節約できました。`);
      }
    }

    return { completedAll, tracked, accuracy, saved, over, messages };
  }, [tasks, period]);

  const periodLabel = period === 'week' ? '今週' : '今月';

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-lg font-semibold">インサイト</h1>
          <div className="flex gap-1">
            {(['week', 'month'] as Period[]).map((p) => (
              <button
                key={p}
                onClick={() => setPeriod(p)}
                className={`px-2.5 py-1 rounded text-2xs font-mono ${
                  period === p
                    ? 'bg-panel2 text-ink'
                    : 'text-mute hover:text-ink'
                }`}
              >
                {p === 'week' ? '今週' : '今月'}
              </button>
            ))}
          </div>
        </div>

        {/* Metrics Cards */}
        <div className="grid grid-cols-2 gap-3 mb-6">
          <div className="border border-border rounded-lg p-4">
            <div className="text-2xs font-mono text-faint uppercase tracking-widest mb-1">完了タスク</div>
            <div className="text-2xl font-semibold">{completedAll.length}</div>
            <div className="text-2xs text-mute">{periodLabel}</div>
          </div>
          <div className="border border-border rounded-lg p-4">
            <div className="text-2xs font-mono text-faint uppercase tracking-widest mb-1">見積もり精度</div>
            <div className="text-2xl font-semibold">
              {tracked.length >= 5 ? `${accuracy}%` : '—'}
            </div>
            <div className="text-2xs text-mute">
              {tracked.length >= 5
                ? accuracy <= 100
                  ? '見積もり以内'
                  : '見積もりオーバー'
                : `あと${Math.max(5 - tracked.length, 0)}件で分析可能`}
            </div>
          </div>
          <div className="border border-border rounded-lg p-4">
            <div className="text-2xs font-mono text-faint uppercase tracking-widest mb-1">節約時間</div>
            <div className="text-2xl font-semibold text-accent">
              {tracked.length >= 5 ? formatMin(saved) : '—'}
            </div>
            <div className="text-2xs text-mute">予定より早く完了</div>
          </div>
          <div className="border border-border rounded-lg p-4">
            <div className="text-2xs font-mono text-faint uppercase tracking-widest mb-1">オーバー時間</div>
            <div className={`text-2xl font-semibold ${over > 0 ? 'text-high' : ''}`}>
              {tracked.length >= 5 ? formatMin(over) : '—'}
            </div>
            <div className="text-2xs text-mute">予定より多くかかった</div>
          </div>
        </div>

        {/* Trend Messages */}
        {messages.length > 0 && (
          <div className="border border-accent/20 bg-accent/5 rounded-lg p-4 mb-6">
            <div className="text-2xs font-mono text-accent uppercase tracking-widest mb-2">フィードバック</div>
            {messages.map((msg, i) => (
              <p key={i} className="text-sm text-ink/80 mb-1 last:mb-0">{msg}</p>
            ))}
          </div>
        )}

        {/* Bar Chart - Estimate vs Actual */}
        {tracked.length > 0 && (
          <div className="border border-border rounded-lg p-4 mb-6">
            <div className="text-2xs font-mono text-faint uppercase tracking-widest mb-4">見積もり vs 実績</div>
            <div className="flex flex-col gap-2">
              {tracked.slice(-10).map((t) => {
                const maxVal = Math.max(t.estimatedMinutes!, t.actualMinutes!, 1);
                const estPct = (t.estimatedMinutes! / maxVal) * 100;
                const actPct = (t.actualMinutes! / maxVal) * 100;
                const isOver = t.actualMinutes! > t.estimatedMinutes!;
                return (
                  <div key={t.id} className="flex flex-col gap-1">
                    <span className="text-2xs font-mono text-mute truncate">{t.title}</span>
                    <div className="flex gap-1 items-center">
                      <span className="text-2xs text-faint w-8 shrink-0 text-right">予</span>
                      <div className="flex-1 h-3 bg-panel2 rounded overflow-hidden">
                        <div
                          className="h-full rounded bg-ink/20"
                          style={{ width: `${estPct}%` }}
                        />
                      </div>
                      <span className="text-2xs font-mono text-faint w-10 text-right">{t.estimatedMinutes}m</span>
                    </div>
                    <div className="flex gap-1 items-center">
                      <span className="text-2xs text-faint w-8 shrink-0 text-right">実</span>
                      <div className="flex-1 h-3 bg-panel2 rounded overflow-hidden">
                        <div
                          className={`h-full rounded ${isOver ? 'bg-high/60' : 'bg-accent/60'}`}
                          style={{ width: `${actPct}%` }}
                        />
                      </div>
                      <span className={`text-2xs font-mono w-10 text-right ${isOver ? 'text-high' : 'text-accent'}`}>{t.actualMinutes}m</span>
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="flex gap-4 mt-3 text-2xs text-faint">
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded bg-ink/20" /> 見積もり</span>
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded bg-accent/60" /> 実績（短縮）</span>
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded bg-high/60" /> 実績（オーバー）</span>
            </div>
          </div>
        )}

        {/* Empty state */}
        {tracked.length === 0 && (
          <div className="border border-border rounded-lg p-8 text-center">
            <div className="text-mute text-sm mb-2">まだデータがありません</div>
            <div className="text-faint text-2xs">
              タスクに見積もり時間を設定して完了すると、ここに分析が表示されます
            </div>
          </div>
        )}

        {tracked.length > 0 && tracked.length < 5 && (
          <div className="border border-border rounded-lg p-4 text-center">
            <div className="text-mute text-sm">
              あと{5 - tracked.length}件の時間記録付きタスクを完了すると、傾向分析ができるようになります
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
