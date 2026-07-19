// Rust バックエンド（src-tauri）への型付き invoke ラッパ。
// DTO は core の serde と共用（D-033）。
import { invoke } from "@tauri-apps/api/core";

export type CellPos = { row: number; col: number };

export type ViolationKind =
  | "MissingRequiredColumn"
  | "DisallowedColumn"
  | "MissingRequiredRow"
  | "DisallowedRow"
  | "TypeMismatch"
  | "NotInEnum"
  | "PatternMismatch"
  | "OutOfRange"
  | "Empty";

export type Violation = {
  sheet: string;
  pos: CellPos | null;
  kind: ViolationKind;
  message: string;
};

export type ConstraintError = { message: string };

export type FileState =
  | "Modified"
  | "Added"
  | "Untracked"
  | "Deleted"
  | "Staged"
  | "Renamed";
export type FileStatus = { path: string; state: FileState };
export type Commit = {
  hash: string;
  message: string;
  author: string;
  timestamp: number;
};

export type HookOutcome =
  | { Ok: string }
  | { Warned: string }
  | { Failed: string }
  | { Skipped: string };

export type OpenResult = {
  sheet_ids: string[];
  git_enabled: boolean;
  constraint_errors: ConstraintError[];
};

export type SheetView = { grid: string[][]; has_row_headers: boolean };
export type SaveResult = {
  violations: Violation[];
  hook_outcomes: HookOutcome[];
};

export const api = {
  openProject: (path: string) => invoke<OpenResult>("open_project", { path }),
  scanCsvTree: () => invoke<string[]>("scan_csv_tree"),
  loadCsv: (sheetId: string) => invoke<SheetView>("load_csv", { sheetId }),
  setCell: (sheetId: string, row: number, col: number, value: string) =>
    invoke<void>("set_cell", { sheetId, row, col, value }),
  saveCsv: (sheetId: string) => invoke<SaveResult>("save_csv", { sheetId }),
  validateSheet: (sheetId: string) =>
    invoke<Violation[]>("validate_sheet", { sheetId }),
  enumOptions: (sheetId: string, row: number, col: number) =>
    invoke<string[] | null>("enum_options", { sheetId, row, col }),
  setTrusted: (yes: boolean) => invoke<void>("set_trusted", { yes }),
  gitStatus: () => invoke<FileStatus[]>("git_status"),
  gitAdd: (files: string[]) => invoke<void>("git_add", { files }),
  gitCommit: (message: string) => invoke<Commit>("git_commit", { message }),
  gitLog: (limit: number) => invoke<Commit[]>("git_log", { limit }),
  runExport: () => invoke<HookOutcome[]>("run_export"),
};
