import { useCallback, useEffect, useState } from "react";
import { api, ConstraintError, Violation } from "./api";
import { ActivityBar, SideView } from "./components/ActivityBar";
import { TreeView } from "./components/TreeView";
import { GitView } from "./components/GitView";
import { TabBar } from "./components/TabBar";
import { Grid } from "./components/Grid";

// レイアウト（D-037）: [アクティビティバー] [サイド: ツリー/git 切替] [タブ+グリッド]
export function App() {
  const [view, setView] = useState<SideView>("tree");
  const [sheetIds, setSheetIds] = useState<string[]>([]);
  const [gitEnabled, setGitEnabled] = useState(false);
  const [constraintErrors, setConstraintErrors] = useState<ConstraintError[]>([]);
  const [openTabs, setOpenTabs] = useState<string[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [violations, setViolations] = useState<Violation[]>([]);

  // デモ: 開発時は URL パラメータ or 固定パスでプロジェクトを開く想定。
  const openProject = useCallback(async (path: string) => {
    const res = await api.openProject(path);
    setSheetIds(res.sheet_ids);
    setGitEnabled(res.git_enabled);
    setConstraintErrors(res.constraint_errors);
  }, []);

  useEffect(() => {
    // GUI 環境では「プロジェクトを開く」ダイアログから設定する（issue I-OPEN-1）。
    // ここでは未接続。
  }, []);

  const openSheet = useCallback((id: string) => {
    setOpenTabs((tabs) => (tabs.includes(id) ? tabs : [...tabs, id]));
    setActive(id);
  }, []);

  const closeTab = useCallback(
    (id: string) => {
      setOpenTabs((tabs) => tabs.filter((t) => t !== id));
      setActive((a) => (a === id ? null : a));
    },
    [],
  );

  return (
    <div className="app">
      <ActivityBar view={view} onChange={setView} gitEnabled={gitEnabled} />
      <aside className="side">
        {view === "tree" ? (
          <TreeView
            sheetIds={sheetIds}
            active={active}
            onOpen={openSheet}
            onOpenProject={openProject}
            constraintErrors={constraintErrors}
          />
        ) : (
          <GitView enabled={gitEnabled} />
        )}
      </aside>
      <main className="editor">
        <TabBar tabs={openTabs} active={active} onSelect={setActive} onClose={closeTab} />
        {active ? (
          <Grid sheetId={active} onViolations={setViolations} />
        ) : (
          <div className="empty">シートを開いてください</div>
        )}
        {violations.length > 0 && (
          <div className="problems">
            {violations.map((v, i) => (
              <div key={i} className="problem">
                ⚠ {v.message}
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
