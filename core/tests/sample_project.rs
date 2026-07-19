//! examples/sample-project を使った統合テスト（実ファイルで設計を圧力テスト）。

use spec_editor_core::Project;
use std::path::PathBuf;

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/sample-project")
}

#[test]
fn opens_sample_and_is_valid() {
    let p = Project::open(sample_path()).expect("open sample project");

    // 再帰スキャンで両シートが見える
    let ids = p.sheet_ids();
    assert!(ids.contains(&"core.csv".to_string()));
    assert!(ids.contains(&"modules/auth.csv".to_string()));

    // 設定ファイルはシート一覧に出ない
    assert!(!ids.iter().any(|i| i.ends_with(".toml")));

    // サンプルは矛盾なし・違反なし
    assert!(p.constraint_errors.is_empty(), "static errors: {:?}", p.constraint_errors);
    let violations = p.validate_all();
    assert!(violations.is_empty(), "unexpected violations: {violations:?}");

    // core.csv のスタイルが読めている（列 processor_type が bold）
    let core = p.sheet("core.csv").unwrap();
    assert_eq!(core.style.columns.get("processor_type").and_then(|a| a.bold), Some(true));
}

#[test]
fn detects_violation_when_edited_invalid() {
    let mut p = Project::open(sample_path()).unwrap();
    // D002 は MIPS なので cores 空欄でも OK だが、processor を ARM に変えると when が成立し cores 必須に
    let s = p.sheet_mut("core.csv").unwrap();
    // 行1（D002）の processor_type 列(index 1) を ARM に
    s.set(spec_editor_core::CellPos { row: 1, col: 1 }, "ARM".into());
    let s = p.sheet("core.csv").unwrap();
    let vs = p.constraints.validate_sheet(s);
    assert!(vs.iter().any(|v| matches!(v.kind, spec_editor_core::ViolationKind::Empty)));
}
