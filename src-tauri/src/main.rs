//! Tauri シェル（薄い配線層, docs/design.md 9.9）。
//!
//! 各コマンドは spec-editor-core のメソッドを呼ぶだけ。ドメインロジックは持たない。
//! 注: この環境（webkit 無し）ではビルド不可。GUI 環境で `cargo tauri dev`（issue I-ENV-1）。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use spec_editor_core::constraints::LineTarget;
use spec_editor_core::git::{Commit, DiffCell, FileStatus};
use spec_editor_core::hooks::HookOutcome;
use spec_editor_core::sheet::CellPos;
use spec_editor_core::validation::{ConstraintError, Violation};
use spec_editor_core::Project;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct AppState(Mutex<Option<Project>>);

type CmdResult<T> = std::result::Result<T, String>;

fn with_project<T>(
    state: &State<AppState>,
    f: impl FnOnce(&mut Project) -> CmdResult<T>,
) -> CmdResult<T> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let proj = guard.as_mut().ok_or("プロジェクトが開かれていません")?;
    f(proj)
}

#[derive(Serialize)]
struct OpenResult {
    sheet_ids: Vec<String>,
    git_enabled: bool,
    constraint_errors: Vec<ConstraintError>,
}

/// プロジェクトを開く。
#[tauri::command]
fn open_project(path: String, state: State<AppState>) -> CmdResult<OpenResult> {
    let proj = Project::open(&path).map_err(|e| e.to_string())?;
    let result = OpenResult {
        sheet_ids: proj.sheet_ids(),
        git_enabled: proj.git_enabled(),
        constraint_errors: proj.constraint_errors.clone(),
    };
    *state.0.lock().map_err(|e| e.to_string())? = Some(proj);
    Ok(result)
}

/// CSV 一覧を再スキャン（design: scan_csv_tree）。
#[tauri::command]
fn scan_csv_tree(state: State<AppState>) -> CmdResult<Vec<String>> {
    with_project(&state, |p| {
        p.scan().map_err(|e| e.to_string())?;
        Ok(p.sheet_ids())
    })
}

#[derive(Serialize)]
struct SheetView {
    grid: Vec<Vec<String>>,
    has_row_headers: bool,
}

/// シートの2次元セルを取得（design: load_csv）。
#[tauri::command]
fn load_csv(sheet_id: String, state: State<AppState>) -> CmdResult<SheetView> {
    with_project(&state, |p| {
        let s = p.sheet(&sheet_id).ok_or("シートが見つかりません")?;
        Ok(SheetView { grid: s.grid.clone(), has_row_headers: s.has_row_headers })
    })
}

/// セルを更新（メモリ上）。row は**データ行** index。
#[tauri::command]
fn set_cell(sheet_id: String, row: usize, col: usize, value: String, state: State<AppState>) -> CmdResult<()> {
    with_project(&state, |p| {
        let s = p.sheet_mut(&sheet_id).ok_or("シートが見つかりません")?;
        s.set(CellPos { row, col }, value);
        Ok(())
    })
}

#[derive(Serialize)]
struct SaveResult {
    violations: Vec<Violation>,
    hook_outcomes: Vec<HookOutcome>,
}

/// 保存フロー（design: save_csv）。違反があっても落ちない（D-031）。
#[tauri::command]
fn save_csv(sheet_id: String, state: State<AppState>) -> CmdResult<SaveResult> {
    with_project(&state, |p| {
        let r = p.save_sheet(&sheet_id).map_err(|e| e.to_string())?;
        Ok(SaveResult { violations: r.violations, hook_outcomes: r.hook_outcomes })
    })
}

/// シート検証（design: validate_sheet）。
#[tauri::command]
fn validate_sheet(sheet_id: String, state: State<AppState>) -> CmdResult<Vec<Violation>> {
    with_project(&state, |p| {
        let s = p.sheet(&sheet_id).ok_or("シートが見つかりません")?;
        Ok(p.constraints.validate_sheet(s))
    })
}

/// enum 候補（プルダウン用, design: enum_options）。
#[tauri::command]
fn enum_options(sheet_id: String, row: usize, col: usize, state: State<AppState>) -> CmdResult<Option<Vec<String>>> {
    with_project(&state, |p| {
        let s = p.sheet(&sheet_id).ok_or("シートが見つかりません")?;
        Ok(p.constraints.enum_options(s, CellPos { row, col }))
    })
}

#[tauri::command]
fn set_trusted(yes: bool, state: State<AppState>) -> CmdResult<()> {
    with_project(&state, |p| {
        p.set_trusted(yes);
        Ok(())
    })
}

// ---- git（.git がある時のみ, D-007） ----

#[tauri::command]
fn git_status(state: State<AppState>) -> CmdResult<Vec<FileStatus>> {
    with_project(&state, |p| match &p.git {
        Some(g) => g.status().map_err(|e| e.to_string()),
        None => Ok(vec![]),
    })
}

#[tauri::command]
fn git_add(files: Vec<String>, state: State<AppState>) -> CmdResult<()> {
    with_project(&state, |p| match &p.git {
        Some(g) => g.add(&files).map_err(|e| e.to_string()),
        None => Err("git 無効".into()),
    })
}

#[tauri::command]
fn git_commit(message: String, state: State<AppState>) -> CmdResult<Commit> {
    with_project(&state, |p| match &p.git {
        Some(g) => g.commit(&message).map_err(|e| e.to_string()),
        None => Err("git 無効".into()),
    })
}

#[tauri::command]
fn git_log(limit: usize, state: State<AppState>) -> CmdResult<Vec<Commit>> {
    with_project(&state, |p| match &p.git {
        Some(g) => g.log(limit).map_err(|e| e.to_string()),
        None => Ok(vec![]),
    })
}

/// セル単位差分（未実装, issue I-DIFF-1）。
#[tauri::command]
fn git_diff(sheet_id: String, state: State<AppState>) -> CmdResult<Vec<DiffCell>> {
    with_project(&state, |p| match &p.git {
        Some(g) => g.diff_cells(&sheet_id).map_err(|e| e.to_string()),
        None => Ok(vec![]),
    })
}

#[tauri::command]
fn run_export(state: State<AppState>) -> CmdResult<Vec<HookOutcome>> {
    with_project(&state, |p| Ok(p.export()))
}

fn main() {
    // LineTarget を use していることを保証（将来の行対象コマンド拡張用）
    let _ = std::mem::size_of::<LineTarget>();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_project,
            scan_csv_tree,
            load_csv,
            set_cell,
            save_csv,
            validate_sheet,
            enum_options,
            set_trusted,
            git_status,
            git_add,
            git_commit,
            git_log,
            git_diff,
            run_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
