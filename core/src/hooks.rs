//! フック（`.hooks.toml`, D-024 / D-027）。
//!
//! `on_save` / `on_export` にコマンドを定義。サブプロセス実行＝コード実行のため
//! **プロジェクト信頼が有効なときだけ実行**（D-027）。CWD=プロジェクトルート、
//! 変数 `{csv_path}` / `{project_root}` / `{output_path}` は絶対パスで渡す。

use crate::error::Result;
use anyhow::Context;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FailPolicy {
    #[default]
    Error,
    Warn,
    Ignore,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hook {
    pub command: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fail_policy: FailPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    Save,
    Export,
}

#[derive(Debug, Default, Deserialize)]
pub struct Hooks {
    #[serde(default)]
    pub on_save: Vec<Hook>,
    #[serde(default)]
    pub on_export: Vec<Hook>,
}

pub struct HookContext {
    pub project_root: PathBuf,
    pub csv_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HookOutcome {
    Ok(String),
    Warned(String),
    Failed(String),
    /// 信頼が無い/コマンド不可などで実行しなかった。
    Skipped(String),
}

impl Hooks {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("hooks 読み込み失敗: {}", path.display()))?;
        let hooks: Hooks =
            toml::from_str(&text).with_context(|| format!("hooks パース失敗: {}", path.display()))?;
        Ok(hooks)
    }

    fn list(&self, event: HookEvent) -> &[Hook] {
        match event {
            HookEvent::Save => &self.on_save,
            HookEvent::Export => &self.on_export,
        }
    }

    /// 指定イベントのフックを順に実行（信頼が必要, D-027）。
    pub fn run(&self, event: HookEvent, ctx: &HookContext, trusted: bool) -> Vec<HookOutcome> {
        let mut outcomes = Vec::new();
        for hook in self.list(event) {
            if !trusted {
                outcomes.push(HookOutcome::Skipped(format!(
                    "未信頼のため未実行: {}",
                    hook.description
                )));
                continue;
            }
            let cmd = expand_vars(&hook.command, ctx);
            match run_shell(&cmd, &ctx.project_root) {
                Ok(true) => outcomes.push(HookOutcome::Ok(hook.description.clone())),
                Ok(false) => outcomes.push(match hook.fail_policy {
                    FailPolicy::Error => HookOutcome::Failed(format!("失敗: {cmd}")),
                    FailPolicy::Warn => HookOutcome::Warned(format!("警告: {cmd}")),
                    FailPolicy::Ignore => HookOutcome::Ok(hook.description.clone()),
                }),
                Err(e) => outcomes.push(HookOutcome::Failed(format!("起動失敗: {cmd} ({e})"))),
            }
        }
        outcomes
    }
}

fn expand_vars(command: &str, ctx: &HookContext) -> String {
    let mut s = command.to_string();
    s = s.replace("{project_root}", &ctx.project_root.display().to_string());
    if let Some(p) = &ctx.csv_path {
        s = s.replace("{csv_path}", &p.display().to_string());
    }
    if let Some(p) = &ctx.output_path {
        s = s.replace("{output_path}", &p.display().to_string());
    }
    s
}

/// シェル経由でコマンドを実行し、終了ステータス成功なら true。
fn run_shell(command: &str, cwd: &Path) -> Result<bool> {
    let status = if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).current_dir(cwd).status()?
    } else {
        Command::new("sh").args(["-c", command]).current_dir(cwd).status()?
    };
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_and_untrusted_skips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(".hooks.toml");
        std::fs::write(
            &p,
            r#"
[[on_save]]
command = "true"
description = "noop"
fail_policy = "error"
"#,
        )
        .unwrap();
        let hooks = Hooks::load(&p).unwrap();
        let ctx = HookContext {
            project_root: dir.path().to_path_buf(),
            csv_path: None,
            output_path: None,
        };
        // 未信頼 → Skipped
        let out = hooks.run(HookEvent::Save, &ctx, false);
        assert!(matches!(out[0], HookOutcome::Skipped(_)));
        // 信頼 → 実行（`true` は成功）
        let out = hooks.run(HookEvent::Save, &ctx, true);
        assert!(matches!(out[0], HookOutcome::Ok(_)));
    }

    #[test]
    fn fail_policy_warn() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(".hooks.toml");
        std::fs::write(
            &p,
            r#"
[[on_save]]
command = "false"
fail_policy = "warn"
"#,
        )
        .unwrap();
        let hooks = Hooks::load(&p).unwrap();
        let ctx = HookContext {
            project_root: dir.path().to_path_buf(),
            csv_path: None,
            output_path: None,
        };
        let out = hooks.run(HookEvent::Save, &ctx, true);
        assert!(matches!(out[0], HookOutcome::Warned(_)));
    }

    #[test]
    fn var_expansion() {
        let ctx = HookContext {
            project_root: PathBuf::from("/proj"),
            csv_path: Some(PathBuf::from("/proj/core.csv")),
            output_path: None,
        };
        let s = expand_vars("python v.py {csv_path} in {project_root}", &ctx);
        assert_eq!(s, "python v.py /proj/core.csv in /proj");
    }
}
