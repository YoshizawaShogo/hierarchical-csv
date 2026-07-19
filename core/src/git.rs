//! Git 操作（git CLI を叩く, D-002）。`.git` があるときのみ有効（D-007）。
//!
//! `diff_cells`（セル単位差分）は最難関で未確定のため未実装スタブ（issue I-DIFF-1）。

use crate::error::{bail, Result};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitRepo {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileStatus {
    pub path: String,
    pub state: FileState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FileState {
    Modified,
    Added,
    Untracked,
    Deleted,
    Staged,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffCell {
    pub row: usize,
    pub col: usize,
    pub old: String,
    pub new: String,
    pub status: CellStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CellStatus {
    Modified,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Commit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

impl GitRepo {
    /// `root` 以下に `.git` があれば Some（D-007）。
    pub fn discover(root: &Path) -> Option<Self> {
        if root.join(".git").exists() {
            Some(Self { root: root.to_path_buf() })
        } else {
            None
        }
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .with_context(|| "git の起動に失敗（git は入っていますか）")?;
        if !out.status.success() {
            bail!("git {:?} 失敗: {}", args, String::from_utf8_lossy(&out.stderr));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// `git status --porcelain` を解析して変更ファイル一覧を返す。
    pub fn status(&self) -> Result<Vec<FileStatus>> {
        let out = self.git(&["status", "--porcelain"])?;
        let mut v = Vec::new();
        for line in out.lines() {
            if line.len() < 3 {
                continue;
            }
            let code = &line[..2];
            let path = line[3..].to_string();
            let state = match code.trim() {
                "??" => FileState::Untracked,
                s if s.starts_with('A') => FileState::Added,
                s if s.starts_with('D') => FileState::Deleted,
                s if s.starts_with('R') => FileState::Renamed,
                s if s.starts_with('M') && code.starts_with('M') => FileState::Staged,
                _ => FileState::Modified,
            };
            v.push(FileStatus { path, state });
        }
        Ok(v)
    }

    /// 複数ファイルを `git add`。
    pub fn add(&self, paths: &[String]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["add", "--"];
        args.extend(paths.iter().map(|s| s.as_str()));
        self.git(&args)?;
        Ok(())
    }

    /// `git commit -m <message>`。作成したコミットを返す。
    pub fn commit(&self, message: &str) -> Result<Commit> {
        self.git(&["commit", "-m", message])?;
        let mut commits = self.log(1)?;
        commits.pop().ok_or_else(|| anyhow::anyhow!("コミット作成後に log が取れません"))
    }

    /// 最新 N 件のコミット。
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        // フィールド区切りに区切り文字を使う（%x1f=US, %x1e=RS）
        let fmt = "%H%x1f%s%x1f%an%x1f%at";
        let out = self.git(&["log", &format!("-{limit}"), &format!("--pretty=format:{fmt}")])?;
        let mut commits = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split('\u{1f}').collect();
            if parts.len() == 4 {
                commits.push(Commit {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    author: parts[2].to_string(),
                    timestamp: parts[3].parse().unwrap_or(0),
                });
            }
        }
        Ok(commits)
    }

    /// セル単位差分（HEAD~1 との比較）。**未実装**（issue I-DIFF-1）。
    pub fn diff_cells(&self, _sheet_id: &str) -> Result<Vec<DiffCell>> {
        bail!("diff_cells は未実装です（issue I-DIFF-1: セル単位差分アルゴリズム未確定）")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").args(args).current_dir(dir).status().unwrap().success();
        assert!(ok, "git {args:?} failed");
    }

    fn init_repo(dir: &Path) {
        run(dir, &["init", "-q"]);
        run(dir, &["config", "user.email", "t@example.com"]);
        run(dir, &["config", "user.name", "t"]);
    }

    #[test]
    fn discover_none_without_git() {
        let dir = tempfile::tempdir().unwrap();
        assert!(GitRepo::discover(dir.path()).is_none());
    }

    #[test]
    fn status_add_commit_log() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join("core.csv"), "a\n1\n").unwrap();

        let repo = GitRepo::discover(dir.path()).expect("repo");
        let st = repo.status().unwrap();
        assert!(st.iter().any(|f| f.path == "core.csv" && f.state == FileState::Untracked));

        repo.add(&["core.csv".to_string()]).unwrap();
        let commit = repo.commit("first").unwrap();
        assert_eq!(commit.message, "first");

        let log = repo.log(5).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "first");
    }
}
