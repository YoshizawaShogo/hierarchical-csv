//! 制約（docs/design.md 第6章 / 9.3）。
//!
//! 2カテゴリのみ: `[[header]]`（行/列名の構造）と `[[value]]`（値の規則）。
//! TOML の `.constraints.toml` から読み込み、静的検証（型×規則の非互換 D-021、
//! header 同士の矛盾 D-012）とシートデータ検証を行う。
//!
//! D-031: 検出したエラー/違反は「表示するデータ」として返し、アプリは落とさない。

pub mod when;

use crate::error::Result;
use crate::glob::GlobPattern;
use crate::sheet::{CellPos, Sheet};
use crate::validation::{ConstraintError, Violation, ViolationKind};
use anyhow::Context;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use when::WhenExpr;

/// 値の型（D-017）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    String,
    Int,
    Float,
}

/// 制約全体。
#[derive(Debug, Default)]
pub struct Constraints {
    pub headers: Vec<HeaderConstraint>,
    pub values: Vec<ValueConstraint>,
}

/// header 制約（1シート1つで行・列を同時定義, D-009 / D-010）。
#[derive(Debug)]
pub struct HeaderConstraint {
    pub sheet: GlobPattern,
    pub column_require: Vec<String>,
    pub column_optional: Vec<GlobPattern>,
    pub row_require: Vec<String>,
    pub row_optional: Vec<GlobPattern>,
}

impl HeaderConstraint {
    pub fn matches_sheet(&self, id: &str) -> bool {
        self.sheet.matches(id)
    }
    /// 列名が require ∪ optional グロブに含まれるか（strict 既定, D-010）。
    pub fn column_allowed(&self, name: &str) -> bool {
        self.column_require.iter().any(|r| r == name)
            || self.column_optional.iter().any(|g| g.matches(name))
    }
    pub fn row_allowed(&self, name: &str) -> bool {
        self.row_require.iter().any(|r| r == name)
            || self.row_optional.iter().any(|g| g.matches(name))
    }
}

/// value 制約（D-009 / D-017 / D-021 / D-026）。
#[derive(Debug)]
pub struct ValueConstraint {
    pub sheet: GlobPattern,
    pub target: LineTarget,
    pub rules: ValueRules,
    pub when: Option<WhenExpr>,
}

/// value 制約の対象ライン（列 or 行）。
#[derive(Debug, Clone, PartialEq)]
pub enum LineTarget {
    Column(String),
    Row(String),
}

#[derive(Debug, Default)]
pub struct ValueRules {
    pub ty: Option<ValueType>,
    pub required: bool,
    pub enums: Option<Vec<String>>,
    pub pattern: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl ValueRules {
    /// 型×規則の互換性（D-021）。非互換なら Err。
    pub fn check_compat(&self) -> std::result::Result<(), String> {
        if self.pattern.is_some() && self.ty.map_or(false, |t| t != ValueType::String) {
            return Err("pattern は type=string のときのみ使えます".into());
        }
        let numeric = matches!(self.ty, Some(ValueType::Int) | Some(ValueType::Float));
        if (self.min.is_some() || self.max.is_some()) && self.ty.is_some() && !numeric {
            return Err("min/max は数値型（int/float）のときのみ使えます".into());
        }
        Ok(())
    }

    /// 1つのセル値がこの規則を満たすか。満たさなければ ViolationKind を返す。
    /// 空欄は `required` に委ねる（issue I-VAL-1）。
    pub fn check_value(&self, value: &str) -> std::result::Result<(), ViolationKind> {
        let v = value.trim();
        if v.is_empty() {
            if self.required {
                return Err(ViolationKind::Empty);
            }
            return Ok(());
        }
        match self.ty {
            Some(ValueType::Int) => {
                if v.parse::<i64>().is_err() {
                    return Err(ViolationKind::TypeMismatch);
                }
            }
            Some(ValueType::Float) => {
                if v.parse::<f64>().is_err() {
                    return Err(ViolationKind::TypeMismatch);
                }
            }
            Some(ValueType::String) | None => {}
        }
        if let Some(enums) = &self.enums {
            if !enums.iter().any(|e| e == v) {
                return Err(ViolationKind::NotInEnum);
            }
        }
        if let Some(pat) = &self.pattern {
            // 暫定: 部分一致（issue I-VAL-3）
            match regex::Regex::new(pat) {
                Ok(re) if re.is_match(v) => {}
                Ok(_) => return Err(ViolationKind::PatternMismatch),
                Err(_) => return Err(ViolationKind::PatternMismatch),
            }
        }
        if self.min.is_some() || self.max.is_some() {
            if let Ok(n) = v.parse::<f64>() {
                if let Some(min) = self.min {
                    if n < min {
                        return Err(ViolationKind::OutOfRange);
                    }
                }
                if let Some(max) = self.max {
                    if n > max {
                        return Err(ViolationKind::OutOfRange);
                    }
                }
            }
        }
        Ok(())
    }
}

impl ValueConstraint {
    pub fn matches_sheet(&self, id: &str) -> bool {
        self.sheet.matches(id)
    }

    /// この制約が指定データ行に適用されるか（when を評価）。
    pub fn applies(&self, sheet: &Sheet, data_row: usize) -> bool {
        match &self.when {
            None => true,
            Some(w) => {
                let cols = sheet.row_map(data_row);
                w.eval(&cols, sheet.row_label(data_row))
            }
        }
    }
}

/// AND 合成済みヘッダー（D-012）。
pub struct EffectiveHeader<'a> {
    pub column_required: BTreeSet<String>,
    pub row_required: BTreeSet<String>,
    matched: Vec<&'a HeaderConstraint>,
}

impl EffectiveHeader<'_> {
    pub fn column_allowed(&self, name: &str) -> bool {
        self.matched.iter().all(|h| h.column_allowed(name))
    }
    pub fn row_allowed(&self, name: &str) -> bool {
        self.matched.iter().all(|h| h.row_allowed(name))
    }
}

impl Constraints {
    /// `.constraints.toml` を読み込む（無ければ空）。
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("constraints 読み込み失敗: {}", path.display()))?;
        let raw: raw::RawConstraints = toml::from_str(&text)
            .with_context(|| format!("constraints パース失敗: {}", path.display()))?;
        raw.into_constraints()
    }

    /// 静的検証: 型×規則の非互換(D-021)・header 矛盾(D-012) を検出して列挙（D-031）。
    pub fn check_static(&self, sheet_ids: &[String]) -> Vec<ConstraintError> {
        let mut errors = Vec::new();
        // value: 型×規則の互換性
        for vc in &self.values {
            if let Err(msg) = vc.rules.check_compat() {
                errors.push(ConstraintError::new(format!(
                    "[value sheet={}] {msg}",
                    vc.sheet.as_raw()
                )));
            }
        }
        // header: シートごとに合成して矛盾検出
        for id in sheet_ids {
            let matched: Vec<&HeaderConstraint> =
                self.headers.iter().filter(|h| h.matches_sheet(id)).collect();
            if matched.len() < 2 {
                continue;
            }
            // ある header が必須とする列を別 header が許可しない → 矛盾（D-012）
            for h in &matched {
                for req in &h.column_require {
                    for other in &matched {
                        if !other.column_allowed(req) {
                            errors.push(ConstraintError::new(format!(
                                "[header sheet={id}] 列 {req:?} が必須だが別の header が禁止（矛盾）"
                            )));
                        }
                    }
                }
                for req in &h.row_require {
                    for other in &matched {
                        if !other.row_allowed(req) {
                            errors.push(ConstraintError::new(format!(
                                "[header sheet={id}] 行 {req:?} が必須だが別の header が禁止（矛盾）"
                            )));
                        }
                    }
                }
            }
        }
        errors
    }

    /// あるシートに効く header を AND 合成（D-012）。
    pub fn effective_header(&self, id: &str) -> EffectiveHeader<'_> {
        let matched: Vec<&HeaderConstraint> =
            self.headers.iter().filter(|h| h.matches_sheet(id)).collect();
        let mut column_required = BTreeSet::new();
        let mut row_required = BTreeSet::new();
        for h in &matched {
            column_required.extend(h.column_require.iter().cloned());
            row_required.extend(h.row_require.iter().cloned());
        }
        EffectiveHeader { column_required, row_required, matched }
    }

    pub fn values_for(&self, id: &str) -> Vec<&ValueConstraint> {
        self.values.iter().filter(|v| v.matches_sheet(id)).collect()
    }

    /// シート全体を検証して違反リストを返す（D-031: 表示用）。
    pub fn validate_sheet(&self, sheet: &Sheet) -> Vec<Violation> {
        let mut out = Vec::new();
        let id = &sheet.id;
        let headers = sheet.column_headers().to_vec();

        // --- header 検証 ---
        let eff = self.effective_header(id);
        for req in &eff.column_required {
            if !headers.iter().any(|h| h == req) {
                out.push(Violation {
                    sheet: id.clone(),
                    pos: None,
                    kind: ViolationKind::MissingRequiredColumn,
                    message: format!("必須の列 {req:?} がありません"),
                });
            }
        }
        if !eff.matched_is_empty() {
            for (ci, h) in headers.iter().enumerate() {
                if !eff.column_allowed(h) {
                    out.push(Violation {
                        sheet: id.clone(),
                        pos: Some(CellPos { row: 0, col: ci }),
                        kind: ViolationKind::DisallowedColumn,
                        message: format!("許可されていない列 {h:?}"),
                    });
                }
            }
        }
        // 行ヘッダー（有効時のみ）
        if sheet.has_row_headers {
            let labels: Vec<String> =
                (0..sheet.data_row_count()).filter_map(|i| sheet.row_label(i).map(String::from)).collect();
            for req in &eff.row_required {
                if !labels.iter().any(|l| l == req) {
                    out.push(Violation {
                        sheet: id.clone(),
                        pos: None,
                        kind: ViolationKind::MissingRequiredRow,
                        message: format!("必須の行 {req:?} がありません"),
                    });
                }
            }
        }

        // --- value 検証 ---
        let col_index: BTreeMap<&str, usize> =
            headers.iter().enumerate().map(|(i, h)| (h.as_str(), i)).collect();
        for vc in self.values_for(id) {
            let LineTarget::Column(col) = &vc.target else {
                continue; // 行対象 value は暫定未実装（issue I-VAL）
            };
            let Some(&ci) = col_index.get(col.as_str()) else { continue };
            for r in 0..sheet.data_row_count() {
                if !vc.applies(sheet, r) {
                    continue;
                }
                let pos = CellPos { row: r, col: ci };
                let val = sheet.get(pos).unwrap_or("");
                if let Err(kind) = vc.rules.check_value(val) {
                    out.push(Violation {
                        sheet: id.clone(),
                        pos: Some(pos),
                        kind,
                        message: format!("列 {col:?} の値 {val:?} が制約に違反"),
                    });
                }
            }
        }
        out
    }

    /// あるセルの enum 候補（プルダウン用）。when を評価し、該当する enum を返す。
    pub fn enum_options(&self, sheet: &Sheet, pos: CellPos) -> Option<Vec<String>> {
        let headers = sheet.column_headers();
        let col = headers.get(pos.col)?;
        for vc in self.values_for(&sheet.id) {
            if let LineTarget::Column(c) = &vc.target {
                if c == col {
                    if let Some(enums) = &vc.rules.enums {
                        if vc.applies(sheet, pos.row) {
                            return Some(enums.clone());
                        }
                    }
                }
            }
        }
        None
    }
}

impl EffectiveHeader<'_> {
    fn matched_is_empty(&self) -> bool {
        self.matched.is_empty()
    }
}

/// TOML の生表現からドメイン型への変換。
mod raw {
    use super::*;

    #[derive(Deserialize)]
    pub struct RawConstraints {
        #[serde(default)]
        header: Vec<RawHeader>,
        #[serde(default)]
        value: Vec<RawValue>,
    }

    #[derive(Deserialize)]
    struct RawHeader {
        sheet: String,
        #[serde(default)]
        column_require: Vec<String>,
        #[serde(default)]
        column_optional: Vec<String>,
        #[serde(default)]
        row_require: Vec<String>,
        #[serde(default)]
        row_optional: Vec<String>,
    }

    #[derive(Deserialize)]
    struct RawValue {
        sheet: String,
        #[serde(default)]
        column: Option<String>,
        #[serde(default)]
        row: Option<String>,
        #[serde(default, rename = "type")]
        ty: Option<ValueType>,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        enum_: Option<Vec<String>>,
        #[serde(default, rename = "enum")]
        enum_kw: Option<Vec<String>>,
        #[serde(default)]
        pattern: Option<String>,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        #[serde(default)]
        when: Option<String>,
    }

    fn globs(v: Vec<String>) -> Result<Vec<GlobPattern>> {
        v.iter().map(|s| GlobPattern::parse(s)).collect()
    }

    impl RawConstraints {
        pub fn into_constraints(self) -> Result<Constraints> {
            let mut headers = Vec::new();
            for h in self.header {
                headers.push(HeaderConstraint {
                    sheet: GlobPattern::parse(&h.sheet)?,
                    column_require: h.column_require,
                    column_optional: globs(h.column_optional)?,
                    row_require: h.row_require,
                    row_optional: globs(h.row_optional)?,
                });
            }
            let mut values = Vec::new();
            for v in self.value {
                let target = match (v.column, v.row) {
                    (Some(c), _) => LineTarget::Column(c),
                    (None, Some(r)) => LineTarget::Row(r),
                    (None, None) => {
                        anyhow::bail!("[value sheet={}] column か row が必要です", v.sheet)
                    }
                };
                let when = match v.when {
                    Some(s) => Some(WhenExpr::parse(&s)?),
                    None => None,
                };
                values.push(ValueConstraint {
                    sheet: GlobPattern::parse(&v.sheet)?,
                    target,
                    rules: ValueRules {
                        ty: v.ty,
                        required: v.required,
                        enums: v.enum_kw.or(v.enum_),
                        pattern: v.pattern,
                        min: v.min,
                        max: v.max,
                    },
                    when,
                });
            }
            Ok(Constraints { headers, values })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet_with(dir: &Path, name: &str, content: &str) -> Sheet {
        std::fs::write(dir.join(name), content).unwrap();
        Sheet::load(dir, name, false).unwrap()
    }

    fn load_constraints(dir: &Path, toml: &str) -> Constraints {
        let p = dir.join(".constraints.toml");
        std::fs::write(&p, toml).unwrap();
        Constraints::load(&p).unwrap()
    }

    #[test]
    fn header_require_and_disallow() {
        let dir = tempfile::tempdir().unwrap();
        let s = sheet_with(dir.path(), "core.csv", "device_id,extra\nD001,x\n");
        let c = load_constraints(
            dir.path(),
            r#"
[[header]]
sheet = "core.csv"
column_require = ["device_id", "processor_type"]
column_optional = ["note"]
"#,
        );
        let vs = c.validate_sheet(&s);
        // processor_type 欠落 + extra が許可外
        assert!(vs.iter().any(|v| v.kind == ViolationKind::MissingRequiredColumn));
        assert!(vs.iter().any(|v| v.kind == ViolationKind::DisallowedColumn));
    }

    #[test]
    fn value_enum_and_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let s = sheet_with(dir.path(), "core.csv", "device_id,processor_type\nD001,ARM\nBAD,SPARC\n");
        let c = load_constraints(
            dir.path(),
            r#"
[[value]]
sheet = "core.csv"
column = "device_id"
type = "string"
pattern = "^D[0-9]{3}$"

[[value]]
sheet = "core.csv"
column = "processor_type"
enum = ["ARM", "MIPS", "x86"]
"#,
        );
        let vs = c.validate_sheet(&s);
        assert!(vs.iter().any(|v| v.kind == ViolationKind::PatternMismatch)); // BAD
        assert!(vs.iter().any(|v| v.kind == ViolationKind::NotInEnum)); // SPARC
    }

    #[test]
    fn value_when_conditional() {
        let dir = tempfile::tempdir().unwrap();
        let s = sheet_with(
            dir.path(),
            "core.csv",
            "processor_type,cores\nARM,\nMIPS,\n",
        );
        let c = load_constraints(
            dir.path(),
            r#"
[[value]]
sheet = "core.csv"
column = "cores"
required = true
when = "{column=processor_type, equals=ARM}"
"#,
        );
        let vs = c.validate_sheet(&s);
        // ARM 行の cores 空欄のみ違反（MIPS 行は when 不成立で対象外）
        assert_eq!(vs.iter().filter(|v| v.kind == ViolationKind::Empty).count(), 1);
    }

    #[test]
    fn static_incompatible_type_rule() {
        let dir = tempfile::tempdir().unwrap();
        let c = load_constraints(
            dir.path(),
            r#"
[[value]]
sheet = "core.csv"
column = "n"
type = "int"
pattern = "x"
"#,
        );
        let errs = c.check_static(&["core.csv".to_string()]);
        assert!(!errs.is_empty()); // int + pattern は非互換（D-021）
    }

    #[test]
    fn enum_options_pulldown() {
        let dir = tempfile::tempdir().unwrap();
        let s = sheet_with(dir.path(), "core.csv", "processor_type\nARM\n");
        let c = load_constraints(
            dir.path(),
            r#"
[[value]]
sheet = "core.csv"
column = "processor_type"
enum = ["ARM", "MIPS", "x86"]
"#,
        );
        let opts = c.enum_options(&s, CellPos { row: 0, col: 0 }).unwrap();
        assert_eq!(opts, vec!["ARM", "MIPS", "x86"]);
    }
}
