//! シート（1枚の CSV）。docs/design.md 9.2。
//!
//! `grid[0]` を列ヘッダー行、`grid[1..]` をデータ行とする。データ行には
//! 表示しない内部 [`RowId`] を割り当て（D-036）、挿入/削除/並べ替えでも
//! 参照がズレないようにする。

use crate::error::Result;
use crate::style::{RowKey, SheetStyle};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// セル位置。`row` は**データ行**の 0 始まり index（ヘッダー行は含まない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellPos {
    pub row: usize,
    pub col: usize,
}

/// 表示しない内部用の行ID（D-036）。挿入/削除/並べ替えでも不変。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RowId(pub u64);

pub struct Sheet {
    pub id: String, // SheetId 文字列（相対パス）
    pub grid: Vec<Vec<String>>,
    pub has_row_headers: bool,
    pub style: SheetStyle,
    row_ids: Vec<RowId>, // データ行（grid[1..]）と並行
    next_row_id: u64,
    dirty: bool,
}

impl Sheet {
    /// CSV とスタイルを読み込み、データ行に RowId を採番する。
    pub fn load(root: &Path, id: &str, has_row_headers: bool) -> Result<Self> {
        let abs = root.join(id);
        let grid = read_csv(&abs)?;
        let data_rows = grid.len().saturating_sub(1);
        let row_ids: Vec<RowId> = (0..data_rows as u64).map(RowId).collect();
        let next_row_id = data_rows as u64;

        // style 読み込み: RowKey(Index/Label) → RowId 解決
        let style = SheetStyle::load(&abs, |k| match k {
            RowKey::Index(i) => row_ids.get(*i).copied(),
            RowKey::Label(l) => {
                if !has_row_headers {
                    return None;
                }
                grid.iter()
                    .skip(1)
                    .position(|r| r.first().map(|c| c == l).unwrap_or(false))
                    .and_then(|i| row_ids.get(i).copied())
            }
        })?;

        Ok(Self { id: id.to_string(), grid, has_row_headers, style, row_ids, next_row_id, dirty: false })
    }

    /// CSV とスタイルを書き出す。
    pub fn save(&mut self, root: &Path) -> Result<()> {
        let abs = root.join(&self.id);
        write_csv(&abs, &self.grid)?;
        // style: RowId → ディスクキー（ラベル優先、無ければ index）
        let has_row_headers = self.has_row_headers;
        let ids = self.row_ids.clone();
        let grid = &self.grid;
        self.style.save(&abs, |id| {
            let idx = ids.iter().position(|r| *r == id)?;
            if has_row_headers {
                if let Some(label) = grid.get(idx + 1).and_then(|r| r.first()) {
                    if !label.is_empty() {
                        return Some(RowKey::Label(label.clone()));
                    }
                }
            }
            Some(RowKey::Index(idx))
        })?;
        self.dirty = false;
        Ok(())
    }

    pub fn column_headers(&self) -> &[String] {
        self.grid.first().map(|r| r.as_slice()).unwrap_or(&[])
    }

    pub fn data_row_count(&self) -> usize {
        self.row_ids.len()
    }

    /// 行ヘッダー有効時のデータ行ラベル（先頭列）。
    pub fn row_label(&self, data_row: usize) -> Option<&str> {
        if !self.has_row_headers {
            return None;
        }
        self.grid.get(data_row + 1).and_then(|r| r.first()).map(|s| s.as_str())
    }

    pub fn get(&self, pos: CellPos) -> Option<&str> {
        self.grid.get(pos.row + 1).and_then(|r| r.get(pos.col)).map(|s| s.as_str())
    }

    pub fn set(&mut self, pos: CellPos, value: String) {
        if let Some(cell) = self.grid.get_mut(pos.row + 1).and_then(|r| r.get_mut(pos.col)) {
            *cell = value;
            self.dirty = true;
        }
    }

    pub fn insert_row(&mut self, at: usize, row: Vec<String>) {
        let at = at.min(self.row_ids.len());
        self.grid.insert(at + 1, row);
        let id = RowId(self.next_row_id);
        self.next_row_id += 1;
        self.row_ids.insert(at, id);
        self.dirty = true;
    }

    pub fn remove_row(&mut self, at: usize) {
        if at < self.row_ids.len() {
            self.grid.remove(at + 1);
            let id = self.row_ids.remove(at);
            self.style.rows.remove(&id);
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn row_id(&self, data_row: usize) -> Option<RowId> {
        self.row_ids.get(data_row).copied()
    }

    pub fn index_of(&self, id: RowId) -> Option<usize> {
        self.row_ids.iter().position(|r| *r == id)
    }

    /// データ行を「列名→値」で見る（when 評価・検証で使用）。
    pub fn row_map(&self, data_row: usize) -> BTreeMap<&str, &str> {
        let mut m = BTreeMap::new();
        let headers = self.column_headers();
        if let Some(r) = self.grid.get(data_row + 1) {
            for (c, name) in headers.iter().enumerate() {
                if let Some(v) = r.get(c) {
                    m.insert(name.as_str(), v.as_str());
                }
            }
        }
        m
    }

    pub fn set_column_attr(&mut self, column: &str, attr: crate::style::Attr) {
        self.style.columns.insert(column.to_string(), attr);
        self.dirty = true;
    }

    pub fn set_row_attr(&mut self, id: RowId, attr: crate::style::Attr) {
        self.style.rows.insert(id, attr);
        self.dirty = true;
    }
}

fn read_csv(path: &Path) -> Result<Vec<Vec<String>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("CSV 読み込み失敗: {}", path.display()))?;
    let mut grid = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        grid.push(rec.iter().map(|s| s.to_string()).collect());
    }
    Ok(grid)
}

fn write_csv(path: &Path, grid: &[Vec<String>]) -> Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("CSV 書き込み失敗: {}", path.display()))?;
    for row in grid {
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn load_and_headers() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "core.csv", "device_id,processor_type\nD001,ARM\nD002,MIPS\n");
        let s = Sheet::load(dir.path(), "core.csv", false).unwrap();
        assert_eq!(s.column_headers(), &["device_id", "processor_type"]);
        assert_eq!(s.data_row_count(), 2);
        assert_eq!(s.get(CellPos { row: 1, col: 1 }), Some("MIPS"));
    }

    #[test]
    fn quoted_fields_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        // カンマ・改行入りセル（csv crate が正しく扱う, D-003）
        write(dir.path(), "q.csv", "name,note\n\"Smith, John\",\"line1\nline2\"\n");
        let mut s = Sheet::load(dir.path(), "q.csv", false).unwrap();
        assert_eq!(s.get(CellPos { row: 0, col: 0 }), Some("Smith, John"));
        assert_eq!(s.get(CellPos { row: 0, col: 1 }), Some("line1\nline2"));
        s.set(CellPos { row: 0, col: 0 }, "Doe, Jane".into());
        s.save(dir.path()).unwrap();
        let s2 = Sheet::load(dir.path(), "q.csv", false).unwrap();
        assert_eq!(s2.get(CellPos { row: 0, col: 0 }), Some("Doe, Jane"));
    }

    #[test]
    fn rowid_stable_across_insert_remove() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "c.csv", "a\n1\n2\n3\n");
        let mut s = Sheet::load(dir.path(), "c.csv", false).unwrap();
        let id_of_2 = s.row_id(1).unwrap(); // 値 "2" の行
        s.insert_row(0, vec!["0".into()]); // 先頭に挿入 → index はズレる
        assert_eq!(s.index_of(id_of_2), Some(2)); // RowId は追従
        s.remove_row(0);
        assert_eq!(s.index_of(id_of_2), Some(1));
    }

    #[test]
    fn row_map_for_when() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "c.csv", "a,b\nx,y\n");
        let s = Sheet::load(dir.path(), "c.csv", false).unwrap();
        let m = s.row_map(0);
        assert_eq!(m.get("a"), Some(&"x"));
        assert_eq!(m.get("b"), Some(&"y"));
    }
}
