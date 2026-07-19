//! グロブパターン（D-022 / D-028）。
//!
//! 標準グロブ（`*`, `?`, `[...]`）に加え、`|` で**丸ごとのパターンを択一**で
//! 区切る（`()` グループ化はしない）。例: `metric_cpu|metric_gpu`。
//! 実装は `|` で分割して複数の [`glob::Pattern`] として保持し、いずれかに
//! マッチすれば真とする。

use crate::error::Result;
use anyhow::Context;

/// `*` 等の標準グロブ + `|` による択一（D-022 / D-028）。
#[derive(Debug, Clone)]
pub struct GlobPattern {
    raw: String,
    alts: Vec<glob::Pattern>,
}

impl GlobPattern {
    /// `raw` を `|` で分割し、それぞれを glob として構築する。
    pub fn parse(raw: &str) -> Result<Self> {
        let mut alts = Vec::new();
        for part in raw.split('|') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let pat = glob::Pattern::new(part)
                .with_context(|| format!("不正なグロブパターン: {part:?}"))?;
            alts.push(pat);
        }
        Ok(Self { raw: raw.to_string(), alts })
    }

    /// いずれかの択一パターンに `text` が一致するか。
    pub fn matches(&self, text: &str) -> bool {
        self.alts.iter().any(|p| p.matches(text))
    }

    /// 元の文字列。
    pub fn as_raw(&self) -> &str {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_star() {
        let g = GlobPattern::parse("metric_*").unwrap();
        assert!(g.matches("metric_cpu"));
        assert!(g.matches("metric_"));
        assert!(!g.matches("cpu_metric"));
    }

    #[test]
    fn alternation_whole_patterns() {
        // D-028: `metric_cpu|metric_gpu`（`metric_(cpu|gpu)` ではない）
        let g = GlobPattern::parse("metric_cpu|metric_gpu").unwrap();
        assert!(g.matches("metric_cpu"));
        assert!(g.matches("metric_gpu"));
        assert!(!g.matches("metric_tpu"));
    }

    #[test]
    fn alternation_with_paths() {
        let g = GlobPattern::parse("modules/auth.csv|modules/db.csv").unwrap();
        assert!(g.matches("modules/auth.csv"));
        assert!(g.matches("modules/db.csv"));
        assert!(!g.matches("modules/search.csv"));
    }

    #[test]
    fn recursive_glob_for_sheets() {
        let g = GlobPattern::parse("**/*.csv").unwrap();
        assert!(g.matches("modules/features/search.csv"));
        assert!(g.matches("core.csv"));
    }

    #[test]
    fn catch_all() {
        let g = GlobPattern::parse("*").unwrap();
        assert!(g.matches("anything"));
    }
}
