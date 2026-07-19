// 開いているシートのタブ（D-037）。未保存の dirty 表示は今後接続（issue）。
export function TabBar(props: {
  tabs: string[];
  active: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
}) {
  const { tabs, active, onSelect, onClose } = props;
  return (
    <div className="tab-bar">
      {tabs.map((id) => (
        <div
          key={id}
          className={`tab ${active === id ? "active" : ""}`}
          onClick={() => onSelect(id)}
        >
          <span className="tab-name">{id.split("/").pop()}</span>
          <span
            className="tab-close"
            onClick={(e) => {
              e.stopPropagation();
              onClose(id);
            }}
          >
            ×
          </span>
        </div>
      ))}
    </div>
  );
}
