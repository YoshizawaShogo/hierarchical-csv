# Spec CSV Editor

ディレクトリ階層化された CSV ファイルの統合管理・編集・バージョン管理を行う
デスクトップアプリ（Tauri + React/TypeScript + Rust）。

- 設計の全体像と決定の経緯: [`docs/design.md`](docs/design.md)（決定ログ D-001〜）
- 実装中に見つけた疑問: [`issue.md`](issue.md)
- ライセンス: MIT + Commons Clause（[`LICENSE`](LICENSE)）

## 構成

```
hierarchical-csv/
├── core/          # ドメイン中核（Tauri 非依存の Rust crate, cargo test で検証可能）
│   └── src/       #   project / sheet / style / constraints(+when,glob) / git / hooks / validation
├── src-tauri/     # Tauri シェル（core を呼ぶ薄いコマンド層）
├── src/           # React フロントエンド（左: ツリー/git 切替、編集: LibreOffice 風グリッド）
├── examples/sample-project/   # 動作確認用サンプル（制約/フック/スタイル入り）
└── docs/design.md
```

型駆動設計（D-029）: まず `core/` に厳密な構造体とメソッドを定義し、
`src-tauri` はそれを呼ぶだけの薄い配線層。

## 開発

```sh
# ドメイン中核のテスト（どの環境でも動く）
cd core && cargo test

# フロントエンドの型チェック / ビルド
npm install
npm run typecheck
npm run build

# デスクトップアプリの起動（要 webkit2gtk 等の GUI 依存, Linux/mac/Win）
npm install
cargo tauri dev      # src-tauri をビルドして起動
```

> 注: CI/コンテナ等で `webkit2gtk` が無い環境では `cargo tauri dev/build` は不可。
> その場合もドメイン中核は `cd core && cargo test` で完全に検証できる（issue I-ENV-1）。

## 現状（Step 0〜1 相当）

- **実装済み・テスト green**: ディレクトリ再帰スキャン、CSV 読み書き（quote/改行対応）、
  Constraint（header/value・when 式・グロブ・静的検査・検証）、内部 RowId、
  スタイルサイドカー、Git（status/add/commit/log）、フック、保存フロー（非破壊 D-031）。
- **骨組みのみ**: フロントエンド UI、Tauri コマンド配線（型チェック/ビルドは通るが GUI 未起動）。
- **未実装**: セル単位の Git 差分（`git_diff`, issue I-DIFF-1）、列操作、信頼プロンプト UI 等。
