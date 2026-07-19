//! 検証結果（docs/design.md 9.8）。
//!
//! D-031: これらは「失敗」ではなく「表示するデータ」。アプリは落とさず UI に出す。

use crate::sheet::CellPos;
use serde::{Deserialize, Serialize};

/// データ（セル値・構造）が制約に違反した箇所。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Violation {
    pub sheet: String, // SheetId の文字列
    pub pos: Option<CellPos>,
    pub kind: ViolationKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationKind {
    MissingRequiredColumn,
    DisallowedColumn,
    MissingRequiredRow,
    DisallowedRow,
    TypeMismatch,
    NotInEnum,
    PatternMismatch,
    OutOfRange,
    Empty,
}

/// 制約ファイル自体の不備（矛盾・型×規則の非互換・パース失敗）。
/// D-031: 表示用データとして返し、アプリは落とさない。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintError {
    pub message: String,
}

impl ConstraintError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
