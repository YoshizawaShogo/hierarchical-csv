import { useCallback, useEffect, useState } from "react";
import { api, Commit, FileStatus } from "../api";

// git ビュー（Source Control 風, D-037）。.git がある時のみ（D-007）。
export function GitView(props: { enabled: boolean }) {
  const { enabled } = props;
  const [status, setStatus] = useState<FileStatus[]>([]);
  const [staged, setStaged] = useState<Set<string>>(new Set());
  const [message, setMessage] = useState("");
  const [log, setLog] = useState<Commit[]>([]);

  const refresh = useCallback(async () => {
    if (!enabled) return;
    setStatus(await api.gitStatus());
    setLog(await api.gitLog(20));
  }, [enabled]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const toggle = (path: string) => {
    setStaged((s) => {
      const n = new Set(s);
      n.has(path) ? n.delete(path) : n.add(path);
      return n;
    });
  };

  const commit = async () => {
    await api.gitAdd([...staged]);
    await api.gitCommit(message);
    setMessage("");
    setStaged(new Set());
    await refresh();
  };

  if (!enabled) return <div className="side-header">git 無効（.git がありません）</div>;

  return (
    <div className="git">
      <div className="side-header">ソース管理</div>
      <input
        className="commit-msg"
        placeholder="コミットメッセージ"
        value={message}
        onChange={(e) => setMessage(e.target.value)}
      />
      <button className="commit-btn" disabled={!message || staged.size === 0} onClick={commit}>
        コミット ({staged.size})
      </button>
      <div className="changes">
        {status.map((f) => (
          <label key={f.path} className="change-row">
            <input type="checkbox" checked={staged.has(f.path)} onChange={() => toggle(f.path)} />
            <span className={`state ${f.state}`}>{f.state[0]}</span> {f.path}
          </label>
        ))}
        {status.length === 0 && <div className="muted">変更なし</div>}
      </div>
      <div className="side-header">履歴</div>
      <div className="log">
        {log.map((c) => (
          <div key={c.hash} className="log-row" title={c.hash}>
            {c.message} <span className="muted">— {c.author}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
