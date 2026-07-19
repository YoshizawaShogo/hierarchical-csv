//! Spec CSV Editor — domain core.
//!
//! Tauri から独立した純粋ドメイン層。`cargo test` で単体検証できる（D-038）。
//! 設計は docs/design.md 第9章「ドメインモデル」に対応。

pub mod constraints;
pub mod error;
pub mod git;
pub mod glob;
pub mod hooks;
pub mod project;
pub mod sheet;
pub mod style;
pub mod validation;

pub use error::Result;
pub use project::{Project, ProjectConfig, SheetId};
pub use sheet::{CellPos, RowId, Sheet};
pub use validation::{Violation, ViolationKind};
