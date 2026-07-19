// 左端のアクティビティバー（D-037）: 階層構造ビュー / git ビュー を切り替え。
export type SideView = "tree" | "git";

export function ActivityBar(props: {
  view: SideView;
  onChange: (v: SideView) => void;
  gitEnabled: boolean;
}) {
  const { view, onChange, gitEnabled } = props;
  return (
    <nav className="activity-bar">
      <button
        className={view === "tree" ? "active" : ""}
        title="階層構造"
        onClick={() => onChange("tree")}
      >
        🗂
      </button>
      <button
        className={view === "git" ? "active" : ""}
        title={gitEnabled ? "ソース管理" : "git 無効（.git なし）"}
        onClick={() => onChange("git")}
        disabled={!gitEnabled}
      >
        ⑂
      </button>
    </nav>
  );
}
