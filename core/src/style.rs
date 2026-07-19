//! 表示属性のサイドカー `<name>.style.toml`（D-035）。
//!
//! CSV には入れず、同ディレクトリの別ファイルに行属性・列属性を持つ。
//! アプリ内では行を [`RowId`](crate::sheet::RowId) で識別し（D-036）、
//! ディスク上は行ラベル or index でアンカーする（[`RowKey`]）。

use crate::error::Result;
use crate::sheet::RowId;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 1つの表示属性セット（行/列単位。将来セル単位も, D-035）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Attr {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub background: Option<String>,
}

/// ディスク上（.style.toml）で行をアンカーする永続キー（D-036）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowKey {
    Label(String),
    Index(usize),
}

/// 表示属性（アプリ内は RowId キー, D-036）。
#[derive(Debug, Clone, Default)]
pub struct SheetStyle {
    pub columns: BTreeMap<String, Attr>,
    pub rows: BTreeMap<RowId, Attr>,
}

/// ディスク表現（TOML）。行は `[row.<label|index>]`、列は `[column.<name>]`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StyleFile {
    #[serde(default)]
    column: BTreeMap<String, Attr>,
    #[serde(default)]
    row: BTreeMap<String, Attr>,
}

impl SheetStyle {
    /// サイドカーファイルのパス（`core.csv` → `core.style.toml`）。
    pub fn sidecar_path(csv_abs: &Path) -> std::path::PathBuf {
        let stem = csv_abs.file_stem().and_then(|s| s.to_str()).unwrap_or("sheet");
        csv_abs.with_file_name(format!("{stem}.style.toml"))
    }

    /// サイドカーを読み込む（無ければ空）。`resolve` で行キー→RowId を解決。
    ///
    /// `resolve` は `RowKey`（Label/Index）を現在の `RowId` に変換する関数。
    pub fn load(
        csv_abs: &Path,
        mut resolve: impl FnMut(&RowKey) -> Option<RowId>,
    ) -> Result<Self> {
        let path = Self::sidecar_path(csv_abs);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("style 読み込み失敗: {}", path.display()))?;
        let file: StyleFile = toml::from_str(&text)
            .with_context(|| format!("style パース失敗: {}", path.display()))?;
        let mut rows = BTreeMap::new();
        for (k, attr) in file.row {
            let key = parse_row_key(&k);
            if let Some(id) = resolve(&key) {
                rows.insert(id, attr);
            }
        }
        Ok(Self { columns: file.column, rows })
    }

    /// サイドカーへ書き出す。`anchor` で RowId→ディスクキーへ変換。
    pub fn save(
        &self,
        csv_abs: &Path,
        mut anchor: impl FnMut(RowId) -> Option<RowKey>,
    ) -> Result<()> {
        let mut file = StyleFile { column: self.columns.clone(), row: BTreeMap::new() };
        for (id, attr) in &self.rows {
            if let Some(key) = anchor(*id) {
                file.row.insert(row_key_string(&key), attr.clone());
            }
        }
        let path = Self::sidecar_path(csv_abs);
        // 全属性が空なら書かない（サイドカーを増やさない）
        if file.column.is_empty() && file.row.is_empty() {
            let _ = std::fs::remove_file(&path);
            return Ok(());
        }
        let text = toml::to_string_pretty(&file)?;
        std::fs::write(&path, text)
            .with_context(|| format!("style 書き込み失敗: {}", path.display()))?;
        Ok(())
    }
}

fn parse_row_key(s: &str) -> RowKey {
    match s.parse::<usize>() {
        Ok(i) => RowKey::Index(i),
        Err(_) => RowKey::Label(s.to_string()),
    }
}

fn row_key_string(k: &RowKey) -> String {
    match k {
        RowKey::Label(s) => s.clone(),
        RowKey::Index(i) => i.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_naming() {
        let p = Path::new("/proj/core.csv");
        assert_eq!(SheetStyle::sidecar_path(p), Path::new("/proj/core.style.toml"));
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("core.csv");
        std::fs::write(&csv, "a,b\n1,2\n").unwrap();

        let mut style = SheetStyle::default();
        style.columns.insert(
            "b".into(),
            Attr { bold: Some(true), color: Some("#c00".into()), ..Default::default() },
        );
        style.rows.insert(RowId(0), Attr { background: Some("#ffd".into()), ..Default::default() });

        // RowId(0) を index 3 でアンカー
        style.save(&csv, |id| Some(RowKey::Index(id.0 as usize + 3))).unwrap();
        assert!(SheetStyle::sidecar_path(&csv).exists());

        // 読み戻し: index 3 → RowId(7) に解決
        let loaded = SheetStyle::load(&csv, |k| match k {
            RowKey::Index(3) => Some(RowId(7)),
            _ => None,
        })
        .unwrap();
        assert_eq!(loaded.columns.get("b").unwrap().bold, Some(true));
        assert_eq!(loaded.rows.get(&RowId(7)).unwrap().background.as_deref(), Some("#ffd"));
    }
}
