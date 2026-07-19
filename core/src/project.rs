//! プロジェクト全体（docs/design.md 9.1）。設定・制約・フック・git・シートを束ねる。
//!
//! D-031: 制約/設定の不備で落とさず、`constraint_errors` に集めて開く。

use crate::constraints::{Constraints, LineTarget};
use crate::error::Result;
use crate::git::GitRepo;
use crate::hooks::{HookContext, HookEvent, HookOutcome, Hooks};
use crate::sheet::Sheet;
use crate::validation::{ConstraintError, Violation};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// シート識別子（ルートからの相対パス, D-006 / D-011）。
pub type SheetId = String;

/// `.project.toml`（D-027）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: String,
}

impl ProjectConfig {
    pub fn load(root: &Path) -> Result<Self> {
        let p = root.join(".project.toml");
        if !p.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&p)?;
        Ok(toml::from_str(&text)?)
    }
}

/// 保存フローの結果（D-031: 落とさず結果を載せて返す）。
pub struct SaveReport {
    pub violations: Vec<Violation>,
    pub hook_outcomes: Vec<HookOutcome>,
}

pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub constraints: Constraints,
    pub hooks: Hooks,
    pub git: Option<GitRepo>,
    pub trusted: bool,
    pub constraint_errors: Vec<ConstraintError>,
    sheets: BTreeMap<SheetId, Sheet>,
}

impl Project {
    /// ルートを開き、設定・制約・フック・git を読み、CSV を再帰スキャンする。
    /// 制約/設定の不備は落とさず constraint_errors に集める（D-031）。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let config = ProjectConfig::load(&root)?;
        let constraints = Constraints::load(&root.join(".constraints.toml")).unwrap_or_default();
        let hooks = Hooks::load(&root.join(".hooks.toml")).unwrap_or_default();
        let git = GitRepo::discover(&root);

        let ids = scan_csv_ids(&root)?;
        let mut sheets = BTreeMap::new();
        for id in &ids {
            let has_row_headers = sheet_has_row_headers(&constraints, id);
            if let Ok(sheet) = Sheet::load(&root, id, has_row_headers) {
                sheets.insert(id.clone(), sheet);
            }
        }
        let constraint_errors = constraints.check_static(&ids);

        Ok(Self {
            root,
            config,
            constraints,
            hooks,
            git,
            trusted: false,
            constraint_errors,
            sheets,
        })
    }

    /// CSV を再帰スキャンしてシート一覧を更新。
    pub fn scan(&mut self) -> Result<()> {
        let ids = scan_csv_ids(&self.root)?;
        // 既存の未保存編集を捨てないよう、無いものだけ追加/消えたものは削除
        self.sheets.retain(|id, _| ids.contains(id));
        for id in &ids {
            if !self.sheets.contains_key(id) {
                let hrh = sheet_has_row_headers(&self.constraints, id);
                if let Ok(sheet) = Sheet::load(&self.root, id, hrh) {
                    self.sheets.insert(id.clone(), sheet);
                }
            }
        }
        self.constraint_errors = self.constraints.check_static(&ids);
        Ok(())
    }

    pub fn sheet_ids(&self) -> Vec<SheetId> {
        self.sheets.keys().cloned().collect()
    }

    pub fn sheet(&self, id: &str) -> Option<&Sheet> {
        self.sheets.get(id)
    }

    pub fn sheet_mut(&mut self, id: &str) -> Option<&mut Sheet> {
        self.sheets.get_mut(id)
    }

    pub fn git_enabled(&self) -> bool {
        self.git.is_some()
    }

    pub fn set_trusted(&mut self, yes: bool) {
        self.trusted = yes;
    }

    /// 保存フロー（7.3 / D-027 / D-031）:
    /// CSV/style を書き込み → 内蔵 Constraint 検証 → on_save フック。
    /// 違反があっても落とさず SaveReport に載せて返す。
    pub fn save_sheet(&mut self, id: &str) -> Result<SaveReport> {
        let root = self.root.clone();
        let sheet = self
            .sheets
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("シートが見つかりません: {id}"))?;
        sheet.save(&root)?;

        let sheet = self.sheets.get(id).unwrap();
        let violations = self.constraints.validate_sheet(sheet);

        let ctx = HookContext {
            project_root: self.root.clone(),
            csv_path: Some(self.root.join(id)),
            output_path: None,
        };
        let hook_outcomes = self.hooks.run(HookEvent::Save, &ctx, self.trusted);

        Ok(SaveReport { violations, hook_outcomes })
    }

    /// エクスポート（on_export フック, D-027）。
    pub fn export(&self) -> Vec<HookOutcome> {
        let ctx = HookContext {
            project_root: self.root.clone(),
            csv_path: None,
            output_path: None,
        };
        self.hooks.run(HookEvent::Export, &ctx, self.trusted)
    }

    /// 全シートを検証（表示用, D-031）。
    pub fn validate_all(&self) -> Vec<Violation> {
        let mut out = Vec::new();
        for sheet in self.sheets.values() {
            out.extend(self.constraints.validate_sheet(sheet));
        }
        out
    }
}

/// このシートが行ヘッダーを持つか（constraints に row_* / Row 対象があるか, D-013）。
fn sheet_has_row_headers(c: &Constraints, id: &str) -> bool {
    let header_hit = c.headers.iter().any(|h| {
        h.matches_sheet(id) && (!h.row_require.is_empty() || !h.row_optional.is_empty())
    });
    let value_hit = c
        .values
        .iter()
        .any(|v| v.matches_sheet(id) && matches!(v.target, LineTarget::Row(_)));
    header_hit || value_hit
}

/// ルート以下を再帰スキャンして CSV の相対パス（`/` 区切り）を集める。
/// `.git/` と設定ファイル（`.constraints.toml` / `.hooks.toml` / `.project.toml` /
/// `*.style.toml`）はシート一覧に出さない（design 5.5）。
fn scan_csv_ids(root: &Path) -> Result<Vec<SheetId>> {
    let mut ids = Vec::new();
    walk(root, root, &mut ids)?;
    ids.sort();
    Ok(ids)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<SheetId>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name == ".git" {
                continue;
            }
            walk(root, &path, out)?;
        } else if name.ends_with(".csv") {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        // *.style.toml / .constraints.toml 等はそもそも .csv ではないので自然に除外
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, content: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn scan_recursive_excludes_config() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join("core.csv"), "a\n1\n");
        write(&r.join("modules/auth.csv"), "b\n2\n");
        write(&r.join("modules/features/search.csv"), "c\n3\n");
        write(&r.join(".constraints.toml"), "");
        write(&r.join("core.style.toml"), "");

        let p = Project::open(r).unwrap();
        let mut ids = p.sheet_ids();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "core.csv".to_string(),
                "modules/auth.csv".to_string(),
                "modules/features/search.csv".to_string()
            ]
        );
    }

    #[test]
    fn save_flow_returns_violations_non_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join("core.csv"), "device_id\nBAD\n");
        write(
            &r.join(".constraints.toml"),
            r#"
[[value]]
sheet = "core.csv"
column = "device_id"
pattern = "^D[0-9]{3}$"
"#,
        );
        let mut p = Project::open(r).unwrap();
        let report = p.save_sheet("core.csv").unwrap(); // 落ちない
        assert!(!report.violations.is_empty()); // BAD が pattern 違反
    }

    #[test]
    fn git_optional() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("core.csv"), "a\n1\n");
        let p = Project::open(dir.path()).unwrap();
        assert!(!p.git_enabled()); // .git 無し
    }
}
