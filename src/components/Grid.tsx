import { useCallback, useEffect, useState } from "react";
import { api, Violation } from "../api";

// LibreOffice(Calc)風グリッド（D-037）: 行番号・列見出し・セル選択・インライン編集。
// grid[0] を列ヘッダー行として扱う（core と同じ規約）。
export function Grid(props: { sheetId: string; onViolations: (v: Violation[]) => void }) {
  const { sheetId, onViolations } = props;
  const [grid, setGrid] = useState<string[][]>([]);
  const [editing, setEditing] = useState<{ r: number; c: number } | null>(null);

  const reload = useCallback(async () => {
    const view = await api.loadCsv(sheetId);
    setGrid(view.grid);
  }, [sheetId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const header = grid[0] ?? [];
  const dataRows = grid.slice(1);

  const commitCell = async (dataRow: number, col: number, value: string) => {
    // データ行 index（ヘッダーを除く）で core に送る
    await api.setCell(sheetId, dataRow, col, value);
    setGrid((g) => {
      const next = g.map((row) => row.slice());
      next[dataRow + 1][col] = value;
      return next;
    });
    setEditing(null);
  };

  const save = async () => {
    const res = await api.saveCsv(sheetId);
    onViolations(res.violations); // 落とさず表示（D-031）
  };

  return (
    <div className="grid-wrap">
      <div className="grid-toolbar">
        <button onClick={save}>保存 (Ctrl/Cmd+S)</button>
        <span className="muted">{sheetId}</span>
      </div>
      <div className="grid-scroll">
        <table className="grid">
          <thead>
            <tr>
              <th className="rownum" />
              {header.map((h, c) => (
                <th key={c} className="colhead">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {dataRows.map((row, r) => (
              <tr key={r}>
                <td className="rownum">{r + 1}</td>
                {header.map((_, c) => {
                  const value = row[c] ?? "";
                  const isEditing = editing?.r === r && editing?.c === c;
                  return (
                    <td
                      key={c}
                      className="cell"
                      onDoubleClick={() => setEditing({ r, c })}
                    >
                      {isEditing ? (
                        <input
                          autoFocus
                          defaultValue={value}
                          onBlur={(e) => commitCell(r, c, e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") commitCell(r, c, (e.target as HTMLInputElement).value);
                            if (e.key === "Escape") setEditing(null);
                          }}
                        />
                      ) : (
                        value
                      )}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
