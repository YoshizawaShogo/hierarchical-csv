//! エラー方針（D-032）: 葉アプリのため `anyhow` を使う。
//!
//! 制約違反・設定不備は「失敗」ではなく「表示するデータ」として
//! `Vec<Violation>` / `Vec<ConstraintError>` で返す（D-031）。ここでの
//! `Result` の Err は真に処理不能な I/O 等に限る。

pub use anyhow::{anyhow, bail, Context, Error, Result};
