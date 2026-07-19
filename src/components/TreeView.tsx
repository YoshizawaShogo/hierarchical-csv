import { useMemo, useState } from "react";
import { ConstraintError } from "../api";

// 階層構造ビュー（Explorer 風ツリー, D-037）。
// フラットな SheetId（相対パス）をフォルダ階層に展開して表示する。
type Node = { name: string; path: string; children: Node[]; isSheet: boolean };

function buildTree(ids: string[]): Node {
  const root: Node = { name: "", path: "", children: [], isSheet: false };
  for (const id of ids) {
    const parts = id.split("/");
    let cur = root;
    let acc = "";
    parts.forEach((part, i) => {
      acc = acc ? `${acc}/${part}` : part;
      const isSheet = i === parts.length - 1;
      let next = cur.children.find((c) => c.name === part);
      if (!next) {
        next = { name: part, path: acc, children: [], isSheet };
        cur.children.push(next);
      }
      cur = next;
    });
  }
  return root;
}

function TreeNode(props: {
  node: Node;
  active: string | null;
  onOpen: (id: string) => void;
  depth: number;
}) {
  const { node, active, onOpen, depth } = props;
  const [open, setOpen] = useState(true);
  const pad = { paddingLeft: `${depth * 12 + 8}px` };
  if (node.isSheet) {
    return (
      <div
        className={`tree-row sheet ${active === node.path ? "active" : ""}`}
        style={pad}
        onClick={() => onOpen(node.path)}
      >
        📄 {node.name}
      </div>
    );
  }
  return (
    <div>
      {node.name && (
        <div className="tree-row folder" style={pad} onClick={() => setOpen(!open)}>
          {open ? "▾" : "▸"} {node.name}
        </div>
      )}
      {open &&
        node.children.map((c) => (
          <TreeNode key={c.path} node={c} active={active} onOpen={onOpen} depth={node.name ? depth + 1 : depth} />
        ))}
    </div>
  );
}

export function TreeView(props: {
  sheetIds: string[];
  active: string | null;
  onOpen: (id: string) => void;
  onOpenProject: (path: string) => void;
  constraintErrors: ConstraintError[];
}) {
  const { sheetIds, active, onOpen, onOpenProject, constraintErrors } = props;
  const tree = useMemo(() => buildTree(sheetIds), [sheetIds]);
  const [path, setPath] = useState("");

  return (
    <div className="tree">
      <div className="side-header">階層構造</div>
      <div className="open-row">
        <input
          placeholder="プロジェクトのパス"
          value={path}
          onChange={(e) => setPath(e.target.value)}
        />
        <button onClick={() => onOpenProject(path)}>開く</button>
      </div>
      {constraintErrors.length > 0 && (
        <div className="constraint-errors">
          {constraintErrors.map((e, i) => (
            <div key={i} className="constraint-error">⚠ {e.message}</div>
          ))}
        </div>
      )}
      <TreeNode node={tree} active={active} onOpen={onOpen} depth={0} />
    </div>
  );
}
