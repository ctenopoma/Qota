import { useRef, useState } from 'react';
import { useTaskStore } from '@/store/useTaskStore';

export default function Settings() {
  const settings = useTaskStore((s) => s.settings);
  const updateSettings = useTaskStore((s) => s.updateSettings);
  const exportJson = useTaskStore((s) => s.exportJson);
  const importJson = useTaskStore((s) => s.importJson);
  const reset = useTaskStore((s) => s.reset);
  const [msg, setMsg] = useState('');

  const flash = (m: string) => {
    setMsg(m);
    setTimeout(() => setMsg(''), 2000);
  };

  const onExport = async () => {
    try {
      const path = await exportJson();
      flash(`保存しました: ${path}`);
    } catch (e) {
      flash('エクスポートに失敗しました');
    }
  };

  const onImport = async () => {
    try {
      await importJson();
      flash('インポートしました');
    } catch (e) {
      flash('インポートに失敗しました');
    }
  };

  const onReset = async () => {
    if (!confirm('サンプルデータに戻します。よろしいですか？')) return;
    await reset();
    flash('リセットしました');
  };

  const card = 'bg-panel border border-border rounded p-4 mb-3 max-w-lg';
  const btn = 'bg-bg border border-border rounded px-3 py-1.5 text-2xs font-mono hover:border-mute';
  const btnPrimary = 'bg-accent text-bg rounded px-3 py-1.5 text-2xs font-mono font-semibold';

  return (
    <div className="overflow-y-auto h-full p-6">
      <h1 className="text-base font-semibold mb-5">設定</h1>

      <div className={card}>
        <div className="text-sm font-medium mb-1">表示モード</div>
        <div className="text-2xs text-mute mb-3">ライト・ダークの切り替え</div>
        <div className="flex gap-1.5">
          <button
            onClick={() => updateSettings({ mode: 'dark' })}
            className={settings.mode === 'dark' ? btnPrimary : btn}
          >
            ● ダーク
          </button>
          <button
            onClick={() => updateSettings({ mode: 'light' })}
            className={settings.mode === 'light' ? btnPrimary : btn}
          >
            ○ ライト
          </button>
        </div>
      </div>

      <div className={card}>
        <div className="text-sm font-medium mb-1">データのエクスポート</div>
        <div className="text-2xs text-mute mb-3">
          すべてのタスク・プロジェクト・タグを JSON で出力します
        </div>
        <button onClick={onExport} className={btn}>
          JSON を保存…
        </button>
      </div>

      <div className={card}>
        <div className="text-sm font-medium mb-1">データのインポート</div>
        <div className="text-2xs text-mute mb-3">
          エクスポートした JSON を読み込んで取り込みます（既存データは上書きされます）
        </div>
        <button onClick={onImport} className={btn}>
          JSON を選択…
        </button>
      </div>

      <div className={card}>
        <div className="text-sm font-medium mb-1">サンプルデータに戻す</div>
        <div className="text-2xs text-mute mb-3">
          すべてのデータを破棄して、初期サンプルに戻します
        </div>
        <button onClick={onReset} className={btn}>
          リセット
        </button>
      </div>

      {msg && <div className="text-2xs font-mono text-accent mt-3">{msg}</div>}
    </div>
  );
}
