# Spec CSV Editor — 設計ドキュメント

> このファイルは「決まったこと」を順次記録していく設計台帳です。
> 議論で1つ決まるたびに、下の **決定ログ** に1件ずつ追記します。
> まだ確定していない項目は **未決事項** に置き、決まったら決定ログへ移します。

- 最終更新: 2026-07-19
- 対象リポジトリ: `yoshizawashogo/hierarchical-csv`
- 作業ブランチ: `claude/spec-csv-editor-ic54h5`

---

## 1. プロジェクト概要（要件仕様書より）

- **目的**: ディレクトリ階層化された CSV ファイルの統合管理・編集・バージョン管理ツール
- **対象ユーザー**: 技術者・非技術者兼用（GUI中心）
- **ライセンス**: MIT + Commons Clause（ツールの販売・SaaS化は禁止／生成CSVは商用OK）

### 技術スタック（仕様書の初期案）

| 層 | 技術 |
|---|---|
| デスクトップ | Tauri |
| フロントエンド | React + TypeScript + Vite |
| バックエンド | Rust |
| Git 操作 | libgit2（`git2` crate）※要検討 |
| CSV 操作 | `csv` crate ※要検討 |

---

## 2. 決定ログ（Decision Log）

> 形式: `YYYY-MM-DD` / 決定番号 / 内容 / 理由。
> ここに載っているものが「確定した仕様」です。

| # | 日付 | 決定内容 | 理由 |
|---|------|----------|------|
| D-000 | 2026-07-19 | 設計は本ファイル（`docs/design.md`）に決定ログ形式で追記していく | 一度に全部を決めず、決まった順に記録するため |
| D-001 | 2026-07-19 | ライセンスは `MIT + Commons Clause`（著作権者: Shogo Yoshizawa）。`LICENSE` を配置 | 仕様書で指定済み。ツールの販売/SaaS化は禁止、生成CSVは商用OK |
| D-002 | 2026-07-19 | Git 操作は `git` CLI のコマンド呼び出し（`std::process::Command`）で実装。libgit2/`git2` crate は使わない | 実装が速く、ビルド/クロスプラットフォームで嵌りにくい。必要になれば後で差し替え可能 |
| D-003 | 2026-07-19 | CSV の読み書きは `csv` crate を使用。行の追加/削除/編集はメモリ上の `Vec<Vec<String>>` で行い、列は動的（`Vec<String>`）扱い。型の担保はコンパイル時の静的型付けではなく実行時のスキーマ検証で行う | クォート/カンマ/改行入りセルを正しく扱える。列構成がCSVごとにユーザー定義のため固定構造体は使えない |
| D-004 | 2026-07-19 | 最初のスキャフォールドのコミットは「Tauri雛形 ＋ Rust⇄TS のIPC型定義（スタブ）＋ 最小 README」を1コミットにまとめる（Q-3 は一任され決定） | 最小の疎通確認単位。LICENSE/設計ドキュメントは配置済みのため別コミット |
| D-005 | 2026-07-19 | 検証方針は Rust=ユニットテスト中心、フロント=Vitest。本環境では Tauri 実バイナリのGUI起動は行わない | この実行環境ではデスクトップGUIの起動確認が難しいため |
| D-006 | 2026-07-19 | 階層モデルは「フォルダ＋シート」。CSVファイル=1枚のシート、ディレクトリ=単なるフォルダ（整理用の入れ物、特別な意味なし）。「1ディレクトリ1CSV」の厳格ルールは採用せず、同一フォルダに複数CSV（複数シート）を許可。全CSVは再帰スキャンで収集 | 複雑なルールを持たせず、表計算に近いシンプルな世界観にするため |
| D-007 | 2026-07-19 | Git はオプショナル（必須にしない）。プロジェクトルートに `.git` が存在する時のみ Git 機能（add/commit/diff/log）を自動有効化。`.git` が無いプロジェクトは Git なしのエディタとして完全動作 | 全ユーザーが Git 管理を望むわけではないため、強制しない |
| D-008 | 2026-07-19 | 旧「スキーマ(schema)」を「**Constraint（制約）**」に呼称変更。フォーマットは **TOML**、ファイルは **`.constraints.toml`** をプロジェクトルートに1つだけ配置。制約は `[[constraints]]`（array-of-tables）で列挙し、対象はテーブル見出しではなく **パラメータ `sheet` / `column` / `row` で指定**（column のみ=列全体、row のみ=行全体、両方=特定セル） | 用語を分かりやすく。TOML は人にもプログラムにも扱いやすい中間形式。行・列の両方を指定できるよう対象を見出しではなくパラメータに置く |
| D-009 | 2026-07-19 | Constraint モデルを網羅版に確定。(1)役割は「設計者(制約を作る)」「エンドユーザー(制約下で編集、転置不要)」の2種。(2)ツールは制約をハードコードせず設計者が自由定義。(3)カテゴリは **`[[header]]` と `[[value]]` の2つのみ**（セクション名＝カテゴリ）。(4)`header` は1シート1つで `column_require/optional`・`row_require/optional` により行・列を同時定義（`axis` 廃止）。(5)`value` は `sheet`＋`column`or`row`＋規則(`required`/`enum`/`pattern`/`min`/`max`/`type`)、条件付きは独立カテゴリにせず `value` 内の **`when`** ガードで表現。(6)各ブロックは `sheet` を先頭に置く | 行/列を別セクションにするとスコープが広がり網羅性が不透明になるため1つに統合。`when` を value 内包にすることで種類の掛け算的増加を防ぎ拡張可能な器にする |
| D-010 | 2026-07-19 | `header` の未指定ラベルは**既定で禁止（明示許可のみ通す）**。`*_require` は**リテラル厳密一致**、`*_optional` は**グロブ可**（`*` / `metric_*`）。「任意の追加を許可」は `optional` に catch-all `*` を入れて表現。`forbid_extra` フラグは廃止 | 既定を厳格にし、許可を列挙／グロブで表せば専用フラグは不要。設計者が余分ラベルを明示制御できる。グロブは `*` の直感に忠実で読みやすい |
| D-011 | 2026-07-19 | 制約の `sheet` は**グロブ展開に対応**（`modules/*.csv`, `**/*.csv` 等）。header/value 共通で、1制約が複数シートに適用されうる | 同種シートへ制約を一括適用でき、記述が簡潔になる |
| D-012 | 2026-07-19 | 同一シートに複数の `[[header]]` が当たる場合は **AND で合成**。矛盾なく同時適用できるならそのまま（必須列=require の和集合、許容列=全 header が許可する列）、**矛盾（一方が必須とする列を他方が禁止 等）があれば制約ロード時にエラー** | グロブ多重適用でも挙動を明確にし、暗黙のどれか採用を排除。矛盾は静的に検出して早期に気付ける |
| D-013 | 2026-07-19 | 行ヘッダー（`row_*`）は **opt-in・例外的**。既定では行は無名。行名が意味を持つ場合（例: 九九）だけ `row_require`/`row_optional` を書き、その場合は**先頭列を行ラベル列**として扱う（データ列ではない） | 大半のシートは行に名前を持たないため、既定を無名にして必要時のみ有効化する |
| D-014 | 2026-07-19 | `when` は**文字列のミニ式**にして簡易パーサで解釈。条件は `{column=名, op=値}`（`row` も可）、連結は `&`(AND)/`|`(OR)、優先順位は `&`>`|`、括弧は初版非対応。比較 `op` は `equals` から開始 | AND/OR の条件付き制約を素直に書ける。TOML インラインテーブルでは `&`/`|` を扱えないため文字列＋自前パースにする |
| D-015 | 2026-07-19 | ~~`when` を2段構えに（L1 宣言式／L2 `when_py` Python）~~ **→ D-026 で `when_py` を撤回。`when` は宣言式のみ** | （撤回。演算子6種＋`&`/`|`/`()` で足り、複雑ケースは on_save フックに切り分けるため） |
| D-016 | 2026-07-19 | L1 宣言式の比較演算子は初版から **`equals` ＋ `not_equal`** の2つ（`gt`/`lt`/`in`/`matches` 等は将来）。あわせて「条件システムは現状ベスト解ではなく改善余地あり（宣言式の拡張＝小言語自作化／`when_py`＝利用者への丸投げの側面）」を設計メモとして明記 | `equals` 単独は貧弱すぎ `not_equal` は必須。ただし演算子を増やし続ける/Python に逃げる路線の限界を認識し、実用最小＋逃げ道で割り切る |
| D-017 | 2026-07-19 | `value` の `type` は **`string` / `int` / `float` の3つのみ**（細分化しない）。規則フィールドの正式セットは `type` / `required` / `enum` / `pattern` / `min` / `max`。**ユーザー定義型は作らない**（検討したが不要と判断） | 型を細かくすると非技術者ユーザーがついていけない。少数固定＋ユーザー定義型なしで複雑さを抑える |
| D-018 | 2026-07-19 | ~~グロブの OR はブレース展開 `{a,b}`~~ **→ D-022 で `\|` に差し替え。** グロブに OR を導入する点自体は有効 | 型を絞った分、パターン表現力をグロブ側で補う |
| D-019 | 2026-07-19 | `when` の比較演算子を **6つに確定**（`equals` / `not_equal` / `gt` / `lt` / `in` / `matches`）、**括弧 `()` に対応**。これで `when` 式まわりは打ち止め | 実用に必要な演算子とグループ化を揃え、条件表現を完成させる |
| D-020 | 2026-07-19 | 同一ライン（列/行）に複数の `[[value]]` が当たる場合、**優先順位は持たない**。全 value を AND 合成し、矛盾なく同時に満たせるかだけを判定。**順序非依存**、矛盾があればエラー | header の合成（D-012）と同じ一貫モデル。優先順位を持たせず判定を単純・予測可能にする |
| D-021 | 2026-07-19 | `type` と規則の互換性を規定。**`pattern` は `string` 時のみ／`min`・`max` は数値（`int`/`float`）時のみ**、`enum`・`required` は任意型。非互換の組合せ（例: `int` かつ `pattern`）は**ロード時エラー** | 意味を成さない組合せを静的に弾き、設計者の誤りを早期に検出する |
| D-022 | 2026-07-19 | グロブの **OR は `\|`** に変更（D-018 のブレース `{a,b}` を差し替え）。~~`()` グループ化~~ **→ D-028 で撤回**。`\|` が直感的 | `\|` の方が択一として直感的、との判断 |
| D-023 | 2026-07-19 | **UI 表示・Git 操作・保存フローは VS Code の作法を参照・リスペクト**し、ほぼ同じ挙動にする（ツリー Explorer、タブ＋dirty 表示、Source Control 風のステージ/コミット、差分ビュー、Ctrl/Cmd+S 保存 等） | 既存の直感を流用でき学習コストが低い |
| D-024 | 2026-07-19 | フック定義は **`.hooks.toml`（TOML）** に置く（旧仕様の config.yaml から変更）。構造はサンプル踏襲（`on_save`/`on_export`、各エントリ command/description/fail_policy、変数 `{csv_path}`/`{project_root}`/`{output_path}`） | Constraint と同じ TOML に統一。細部は Q-5〜Q-9 で詰める |
| D-025 | 2026-07-19 | `enum` は **C 言語レベルの単純な列挙**（許可値の固定セットのみ）。付加情報・高度機能は持たせない | 非技術者にも分かる最小の意味論に留める |
| D-026 | 2026-07-19 | **`when_py`（Python エスケープハッチ）は不採用**（D-015 を撤回）。`when` は宣言式のみ。任意コードは実行しない。複雑条件で宣言式に収まらないものは `.hooks.toml` の `on_save` フックで独自検証する | 演算子6種＋`&`/`|`/`()` で実用上足りる。任意コード実行の面をエディタのセル検証から排除できる |
| D-027 | 2026-07-19 | フックの挙動を確定（Q-5〜Q-9）。①内蔵 Constraint 検証 → 通れば `on_save`（フックは追加処理）②プロジェクト信頼が有効な時のみ実行③`fail_policy`=`error`/`warn`/`ignore`④CWD=プロジェクトルート・変数は絶対パス⑤プロジェクト設定は **`.project.toml`**（設定/制約/フックの3ファイル体制） | サブプロセス実行の安全・順序・パスを明確化し、設定ファイルを役割ごとに分離 |
| D-028 | 2026-07-19 | グロブ OR の `\|` は **丸ごとのパターンを択一で区切る**（`()` グループ化はしない）。例: `metric_cpu\|metric_gpu`（`metric_(cpu\|gpu)` ではない） | 期待挙動に合わせ、パーサも `\|` で分割するだけの単純実装にする |
| D-029 | 2026-07-19 | 実装は **型駆動設計**で進める。まず厳密な構造体と、その振る舞いをメソッドとして定義（第9章 ドメインモデル）。以降は薄いヘルパと Tauri コマンドの配線でアプリを完成させる | 構造体と振る舞いが正しく定義できていれば接続は機械的で済み、破綻を早期に発見できる |
| D-030 | 2026-07-19 | `.constraints.toml` / `.hooks.toml` / `.project.toml` を**アプリ内の専用エディターで編集**できるようにする | ユーザーが仕様に沿って書く必要があり、GUI 内で完結させるため |
| D-031 | 2026-07-19 | 制約違反・設定不備で**アプリを落とさない**。必ず**開いた上でエラーを UI にインライン表示**する（設定ファイルのパース/矛盾エラーも、データのセル違反も同様）。保存も違反で中断せず結果を表示 | 途中で落とすと対話型 GUI エディターの利点が失われるため。エラーは"失敗"でなく"表示するデータ"として扱う |
| D-032 | 2026-07-19 | エラー型は **`anyhow`**（ライブラリ用の厳密エラー型 `thiserror` 等は作らない） | 本アプリは依存グラフの葉（最終成果物）でライブラリとして使われないため |
| D-033 | 2026-07-19 | IPC の DTO は**基本ドメイン構造体に `serde` を直接 derive して共用**、そのまま出せない箇所（ライフタイム付き等）だけ別 DTO を用意 | 葉アプリなので変換コストを最小化しつつ、必要箇所のみ分離する |
| D-034 | 2026-07-19 | 依存方針: **`clap` 等の CLI 引数解析 crate は不要**（GUI アプリ）。DTO は `serde` でそのまま渡す（D-033 を確認） | コマンドライン引数を扱わないため。シリアライズは serde で十分 |
| D-035 | 2026-07-19 | 表示属性（フォント/強調/色 等）は CSV に入れず、**同ディレクトリのサイドカー `<シート名>.style.toml`** に保持。**行属性・列属性**を持ち描画時に適用。アプリが読み書きし、シート一覧には出さない | CSV にスタイルを持たせるとデータサイズが膨らむため分離。TOML で他設定と統一 |
| D-036 | 2026-07-19 | **表示しない内部用 `RowId`** を導入し行の同一性を管理。ロード時に採番、追加時に新規採番（セッション内一意・再利用なし）。スタイル行属性・制約の行対象・選択/undo 等の内部参照はこの `RowId` を使い、挿入/削除/並べ替えでズレないようにする。**永続化は既定で非永続**（サイドカーは保存時に index/行ラベルでアンカーし、ロード時に振り直す） | 「行の同一性」問題を一箇所で解消。アプリ内編集中のズレを根絶しつつ、CSV を汚さない |
| D-037 | 2026-07-19 | UI レイアウト: **左に「階層構造ビュー」と「git ビュー」を切り替え表示**（VS Code のアクティビティバー的に切替可能）。**編集エリア（グリッド）はLibreOffice(Calc)風**。 | ツリー/git を左で切り替える馴染んだ形＋表計算の直感を流用 |
| D-038 | 2026-07-19 | 実装アーキテクチャ: ドメイン中核を **Tauri 非依存の独立 crate `core/`** として実装（`cargo test` で検証可能）。`src-tauri/` は core を呼ぶ薄い Tauri シェル。フロントは `src/`（React/TS/Vite） | webkit 非搭載環境でも中核をビルド/テストでき、型駆動の核を GUI から独立に固められる（D-005/D-029 と整合） |

<!--
追記テンプレート（コピーして使う）:
| D-00X | YYYY-MM-DD | （決めた内容） | （なぜそう決めたか） |
-->

---

## 3. 未決事項（Open Questions）

> 決まったら決定ログへ移動します。優先度の高いものから並べています。

現在、未決事項はありません（Q-1〜Q-9 はすべて決定ログへ移動済み）。Constraint／フック／設定ファイルの設計は一通り確定し、次は Step 0（スキャフォールド）に進める状態です。

---

## 4. 実装順（ドラフト）

> 依存関係とリスクの高さで並べた案。確定したら決定ログに反映します。

- **Step 0**: 骨組みとデータ契約（Tauri雛形 / Rust⇄TS のIPC型定義 / 疎通確認）
- **Step 1**: 読み取り専用表示（`scan_csv_tree` → `load_csv` → タブ＋テーブル表示のみ）
- **Step 2**: 編集と保存（インライン編集 / `save_csv`。フックはまだ呼ばない）
- **Step 3**: Git 最小ループ（`git_status`/`git_add`/`git_commit` ＋ コミットダイアログ）→ ここでMVP完結
- **Step 4**: 差分表示（`git_diff` のセル単位パース／#fff3cd ハイライト）← 最難関
- **Step 5**: `git_log` 表示
- **Step 6**: フック機構（`config.yaml` → `on_save` subprocess）
- **Step 7**: Constraint 検証（`.constraints.toml`。enum プルダウン先行 → pattern/min-max は後）

---

## 5. 階層構造モデル（フォルダ＋シート）

> D-006 / D-007 に基づく定義。

### 5.1 基本モデル

- **CSVファイル = 1枚の「シート」**。表計算ソフトのシート名のような扱い。
- **ディレクトリ = 単なるフォルダ**。整理のための入れ物で、特別な意味は持たせない。
- 同一フォルダ内に複数のCSV（複数シート）を置いてよい（「1ディレクトリ1CSV」の強制はしない）。
- プロジェクトルート以下を**再帰的にスキャン**して、全CSV（全シート）を収集する。

```
spec-project/            ← プロジェクトルート（フォルダ）
├── core.csv             ← シート
├── modules/             ← フォルダ（整理用）
│   ├── auth.csv         ← シート
│   ├── database.csv     ← シート
│   └── features/        ← フォルダ
│       ├── search.csv         ← シート
│       └── notifications.csv  ← シート
└── config/
    └── versions.csv     ← シート
```

### 5.2 識別子

- 各シートは**プロジェクトルートからの相対パス**で識別する（例: `modules/features/search.csv`）。
- この相対パスを UI・Git 操作・フックの `{csv_path}` で共通のキーとして用いる。

### 5.3 Git の扱い（オプショナル）

- Git は**必須ではない**。
- プロジェクトルートに **`.git` が存在する場合のみ**、Git 関連機能（`git add` / `commit` / `diff` / `log`）を自動的に有効化する。
- `.git` が無い場合は、Git 機能を無効（非表示/無効化）にし、**純粋な CSV エディタ**として動作する。
- ユーザーが後から `git init` した場合、その状態を検知して Git 機能が有効になる（起動時 or リフレッシュ時に判定）。

### 5.4 表示属性のサイドカー（`<シート名>.style.toml`, D-035）

- 各シートの**表示属性（フォント・太字・色・背景 等）**は、CSV 本体ではなく**同ディレクトリのサイドカー**に保持する（例: `core.csv` → `core.style.toml`）。CSV のデータサイズを膨らませないため。
- 保持するのは **行に対する属性**と**列に対する属性**（当面。セル単位は将来）。
- **アプリが読み書き**（GUI の書式操作から）。手書き前提ではない。スキャンではシート一覧に出さない。
- 行属性の内部キーは**隠し `RowId`**（D-036）で管理し、挿入/削除/並べ替えでもズレない。**ディスク上（`.style.toml`）は行ラベル or index でアンカー**し、ロード時に `RowId` へ振り直す。

```toml
# core.style.toml（core.csv と同じディレクトリ）

[column.processor_type]      # 列に対する属性
bold  = true
color = "#c00000"
font  = "monospace"

[row.3]                      # 行に対する属性（行ラベル or index）
background = "#fffbdd"
```

### 5.5 未確定（このモデルに紐づく後続論点）

- スキャン除外ルール（`.git/` は当然除外。`.constraints.toml` / `.hooks.toml` / `.project.toml` / `*.style.toml` はシート一覧に出さない）
- UI 上でフォルダ階層をどう見せるか（ツリー / タブグループ / フラット）。データモデルは「フォルダ＋シート」で確定だが、**表示方法は別途 UI 設計で決める**。

---

## 6. Constraint（制約）定義

> D-008 / D-009 に基づく定義。旧称「スキーマ(schema)」。

### 6.1 役割モデル

制約に関わるツール利用者は2種類:

- **設計者（制約を作る人）** … `.constraints.toml` を書く人。
- **エンドユーザー（制約を使う人）** … その制約下でデータ（セル）を編集する人。転置（行↔列の入れ替え）機能は不要。

**ツール側は制約をあらかじめ組み込まない（ハードコードしない）。** 汎用の仕組みだけを提供し、設計者が自由に・いくらでも制約を定義できるようにする。

### 6.2 ファイル

- **プロジェクトルートに `.constraints.toml` を1つだけ**配置する（プロジェクト全体で1ファイル）。
- フォーマットは **TOML**（人・プログラム双方に扱いやすい中間形式）。
- 制約は**後付け可能**（無くてもエディタは動作する。あれば検証・プルダウン等に使う）。指定が無い箇所は自由。

### 6.3 2つのカテゴリ

制約は **`[[header]]`** と **`[[value]]`** の2カテゴリのみ。TOML のセクション名がカテゴリを表す。

- **`[[header]]`** … 行名・列名（構造）の制約。**1シートにつき1つの `header` で、行・列の両方**を定義する（`axis` は使わず `column_*` / `row_*` で書き分ける。網羅性が一目で分かる）。
- **`[[value]]`** … セルの値への制約。`enum` / `pattern` / `required` / 範囲 などはすべて `value` の中のフィールドとして表現。独立カテゴリにはしない。

いずれも各ブロックの先頭は `sheet`（見栄えのため `sheet` を最初に置く）。

### 6.4 `[[header]]` の記法

| フィールド | 意味 |
|---|---|
| `sheet` | 対象シートの相対パス（必須）。**グロブ可**（`modules/*.csv`, `**/*.csv`）→ 複数シートに適用されうる |
| `column_require` | 必須の列名リスト。**リテラルで厳密一致** |
| `column_optional` | 任意（あってもよい）列名リスト。**グロブ可**（`*` / `metric_*`） |
| `row_require` | 必須の行名リスト。**リテラルで厳密一致** |
| `row_optional` | 任意の行名リスト。**グロブ可** |

**未指定ラベルの扱い（重要）:**
- `require` にも `optional` にも当てはまらないラベルは **既定で禁止（エラー）**。明示的に許可されていないものは全部アウト。
- 「任意の追加ラベルを許可」したい場合は、`optional` に catch-all の **`*`** を1つ入れる。
- `require` は**リテラル厳密一致**（必須名は確定しているため）。グロブが使えるのは `optional` 側のみ。
- 上記により、旧案の `forbid_extra` フラグは**不要（廃止）**。既定が厳格で、許可は列挙／グロブで表す。

**グロブ記法（全グロブ箇所に共通: `sheet` / `*_optional`）:**
- `*`（任意文字列）等の標準グロブに加え、**OR は `|`** で表す。`|` は**丸ごとのパターンを択一で区切る**（`()` によるグループ化はしない）。
  - 例: `metric_cpu|metric_gpu` → `metric_cpu` / `metric_gpu`、`modules/auth.csv|modules/db.csv` → 2ファイルに適用。
  - 実装: `|` は標準グロブ機能ではないため、`|` で分割して複数の具体グロブとして扱う。

**行ヘッダー（`row_*`）は opt-in・例外的:**
- 多くのシートは行に名前を持たない（行はただのデータ）。**既定では行は無名**。
- 行名が意味を持つ場合（例: 九九の表。左端が 1〜9 の行ラベル）だけ `row_require` / `row_optional` を書く。
- **`row_*` を書いたシートは「行ヘッダー有り」** とみなし、**先頭列を行ラベル列**として扱う（その列はデータ列ではなくラベル）。`row_*` が無ければ先頭列は通常のデータ列。

### 6.5 `[[value]]` の記法

**対象指定:**

| フィールド | 意味 |
|---|---|
| `sheet` | 対象シートの相対パス（必須）。**グロブ可**（複数シートに適用されうる） |
| `column` / `row` | 対象ライン。列を狙うなら `column`、行を狙うなら `row`（`axis` は廃止） |
| `when` | **適用条件（ガード）**。この value 制約を、条件に一致する行にだけ適用する。**文字列のミニ式**（→ 6.7）|

**規則フィールド（正式セット。これ以上は増やさない方針）:**

| フィールド | 意味 |
|---|---|
| `type` | 値の型。**`string` / `int` / `float` の3つのみ**（細分化しない） |
| `required` | 空欄を禁止（値が必須） |
| `enum` | 許可値の列挙リスト（`type` とは別軸で使える）。**C 言語レベルの単純な列挙**＝許可値の固定セットのみ（付加情報・高度機能は持たない） |
| `pattern` | 正規表現に一致（主に `string`） |
| `min` / `max` | 数値範囲（`int` / `float`） |

- **ユーザー定義型は作らない**（組み込みの少数型のみ。複雑さを避ける）。
- **条件付き制約は独立カテゴリにしない。** 「ある条件のときだけ適用される value 制約」＝ `value` に `when` を付けたもの、として扱う。
- 特定セル1つを狙う場合も「`column`（列）＋ `when`（行の条件）」で表現できる。

**`type` と規則の互換性（非互換はロード時エラー）:**

| 規則 | 使える `type` |
|---|---|
| `pattern` | `string` のときのみ |
| `min` / `max` | 数値（`int` / `float`）のときのみ |
| `enum` / `required` | 任意の型で可 |

- 非互換の組み合わせ（例: `type=int` かつ `pattern`）は**制約ロード時にエラー**にする。

### 6.6 網羅版の例

```toml
# .constraints.toml（プロジェクトルートに1つ）

# ── header: 行名・列名（構造）。1シート1つで行も列も定義 ──
[[header]]
sheet           = "core.csv"
column_require  = ["device_id", "processor_type"]  # 必須の列名（リテラル厳密一致）
column_optional = ["note", "owner", "metric_*"]    # 任意の列名（グロブ可）。metric_* を許可
row_require     = []                               # 必須の行名（不要なら空/省略）
row_optional    = []                               # 任意の行名（グロブ可）
# require にも optional にも当てはまらない列 → エラー（既定=禁止）
# 任意の追加列をすべて許可したいなら column_optional に "*" を入れる

# ── value: 値への制約 ──
[[value]]
sheet  = "core.csv"
column = "processor_type"
enum   = ["ARM", "MIPS", "x86"]

[[value]]
sheet    = "core.csv"
column   = "device_id"
required = true
pattern  = "^D[0-9]{3}$"

# ── 条件付き: when を value の中に含める ──
[[value]]
sheet    = "core.csv"
column   = "cores"
required = true
enum     = ["1", "2", "4"]
when     = "{column=processor_type, equals=ARM}"
# → processor_type が "ARM" の行のときだけ、この cores 制約を適用

# 複合条件（AND / OR）
[[value]]
sheet    = "core.csv"
column   = "cooling"
required = true
when     = "{column=processor_type, equals=ARM} & {column=voltage, equals=high}"
```

### 6.7 `when` 式の文法（簡易パーサ）

`when` は TOML のインラインテーブルではなく、**自前で解釈する文字列**（`&` / `|` を使うため）。

- **条件**: 波括弧 `{ ... }`。中身は `{column=<名>, <op>=<値>}`（例: `{column=processor_type, equals=ARM}`）。`column` の代わりに `row` も可。
- **連結**: `&`（AND）と `|`（OR）。
- **括弧 `()` 対応**。`&`/`|` の優先順位を明示的にグループ化できる。既定の優先順位は `&` が `|` より強い（一般的なブール式と同じ）。
- 比較演算子（`op`）は **6つ**: `equals` / `not_equal` / `gt` / `lt` / `in` / `matches`。
  - `gt` / `lt` … 数値比較（`>` / `<`）
  - `in` … リストのいずれかに一致（例: `{column=type, in=[A,B,C]}`）
  - `matches` … 正規表現に一致
- 文法（EBNF 概略）:
  ```
  expr   := term ( "|" term )*
  term   := factor ( "&" factor )*
  factor := cond | "(" expr ")"
  cond   := "{" key "=" val ( "," key "=" val )* "}"
  ```

```toml
when = "{column=type, equals=A} | {column=type, equals=B}"       # A または B
when = "{column=a, equals=1} & {column=b, equals=2}"             # a=1 かつ b=2
when = "{column=status, not_equal=deprecated}"                   # deprecated 以外
when = "({column=a, equals=1} | {column=a, equals=2}) & {column=b, gt=10}"  # 括弧でグループ化
when = "{column=cores, in=[2,4,8]}"                             # in
```

> **設計メモ**: `when` は**宣言式のみ**（任意コードは実行しない）。かつて Python エスケープハッチ `when_py` を検討したが**不採用**（D-026）。複雑な条件は上記6演算子＋`&`/`|`/`()` で表現し、それでも足りないケースは `.hooks.toml` の `on_save` フックで独自検証する、という切り分けにする。

### 6.8 実装スコープ

- **最初（Phase 1〜2）**: `header` と、`value` の単純な規則（`required` / `enum` / `pattern` / `min` / `max` / `type`）を実装。
- **後から**: `value` の `when`（宣言式の条件付き）を追加。カテゴリ（`header`/`value`）は変えず、`value` にフィールドを足すだけで拡張できる器にしておく。

### 6.9 複数の制約が同じシートに当たる場合（合成と矛盾検出）

`sheet` がグロブ対応のため、1つのシートに複数の `[[header]]` が当たることがある。その合成規則:

- 当たった全 `[[header]]` を **AND（すべて同時に満たすべき表明）** として合成する。
- **矛盾なく同時適用できるなら、合成結果をそのまま適用**する。
  - 必須列 = 各 `require` の**和集合**（すべて存在必須）。
  - 許容列 = 当たった**すべての header** が許可（`require` か `optional` グロブに一致）する列のみ。
- **矛盾があれば、制約ロード時に検出して報告**する（黙って一方を採用しない）。ただし D-031 によりアプリは落とさず、**エラーを設定エディターに表示**して修正を促す。
  - 矛盾の定義: ある header が**必須**とする列を、別の header が**許可していない**（strict 既定で暗黙禁止）→ 「存在必須」かつ「存在禁止」で同時に満たせない。

```toml
# 矛盾の例 → ロード時エラー
[[header]]
sheet          = "*.csv"          # 全シート、id 以外は禁止
column_require = ["id"]

[[header]]
sheet          = "core.csv"       # core.csv にも当たる
column_require = ["id", "name"]   # name 必須
# → core.csv では前者が name を禁止・後者が name を必須 = 矛盾
```

**複数の `[[value]]` が同一ライン（列/行）に当たる場合も同じ考え方:**
- **優先順位という概念は持たない。** 該当する全 value を **AND で合成**し、すべてを矛盾なく同時に満たせるかだけを見る。
- **順序非依存**（`[[value]]` の記述順を入れ替えても結果は同じ）。
- 同時に満たせない矛盾（例: 同じ列に `type=int` と `type=string`、あるいは両立しない `enum`）があれば**エラー**。

### 6.10 未確定（後続で詰める）

- （Constraint 周りの主要論点は D-008〜D-025 で確定済み。残課題が出たらここに追記する）
- 例: `in` / `matches` の値表記の細部（リスト記法・正規表現のエスケープ）などの実装時の細目。

---

## 7. UI・Git・保存フロー（VS Code 準拠）

> D-023 に基づく。UI 表示・Git 操作・保存フローは **VS Code の作法を参照・リスペクト**し、ほとんど同じ挙動にする（既存の直感を流用して学習コストを下げる）。細部は実装時に VS Code の挙動へ寄せる。

### 7.1 UI 表示（D-037）
- **左サイドバーは「階層構造ビュー」と「git ビュー」を切り替え**（VS Code のアクティビティバー的）。
  - 階層構造ビュー: フォルダ階層を **Explorer 風のツリー**（フォルダ＝ディレクトリ、葉＝シート/CSV）。
  - git ビュー: Source Control 風（変更一覧・ステージ・コミット）。`.git` がある時のみ（D-007）。
- 開いたシートは**タブ**表示。未保存は**タブにドット（dirty 表示）**。
- **編集エリア（グリッド）は LibreOffice(Calc)風**（行番号・列見出し・セル選択・インライン編集）。

### 7.2 Git 操作（`.git` がある時のみ = D-007）
- **Source Control パネル**風。変更ファイル一覧、**チェックボックスでステージ/アンステージ**、コミットメッセージ入力、コミットボタン。
- 差分は **inline / side-by-side の差分ビュー**。セル単位ハイライト（#fff3cd）は本ツール固有の上乗せ。

### 7.3 保存フロー
- **Ctrl/Cmd+S で保存**、dirty 表示。
- 順序: **CSV 書き込み → 内蔵 Constraint 検証 → `on_save` フック**。違反があっても**保存を落とさず**、違反を UI にインライン表示する（D-031）。`on_save` フックの `fail_policy` は設計者が選ぶ強度（結果は UI に表示、アプリは落ちない）。

### 7.4 設定ファイルの内蔵エディターと非破壊なエラー表示（D-030 / D-031）
- `.constraints.toml` / `.hooks.toml` / `.project.toml` は**アプリ内の専用エディター**で編集できる（ユーザーが仕様に沿って書く必要があるため）。
- これらに不備（パースエラー、header 矛盾 D-012、型×規則の非互換 D-021 等）があっても、**アプリは落とさず**、エディター上に**エラーをインライン表示**して修正を促す。
- データ（セル値）が制約に違反していても同様に、**シートは開いた上で**該当セルにエラーを表示（対話型 GUI エディターの利点を殺さない）。

---

## 8. フック（`.hooks.toml`）

> D-024 / D-027 に基づく。旧仕様の `config.yaml` から **`.hooks.toml`（TOML）** に変更。構造はサンプル踏襲。

```toml
# .hooks.toml（プロジェクトルート）

[[on_save]]
command     = "python validate_extra.py {csv_path}"
description = "追加の独自チェック"
fail_policy = "error"          # error（中止）/ warn（警告のみ）/ ignore

[[on_export]]
command     = "python gen_html.py {project_root} out/spec.html"
description = "HTML 仕様書生成"
```

**挙動（D-027 で確定）:**
- **役割分担・順序**: 保存時はまず**内蔵 Constraint 検証**が走り、**通れば `on_save` フック**を実行。フックは内蔵検証とは別の**追加のカスタム処理**（コード生成・独自チェック等）。
- **信頼モデル**: フックはサブプロセス実行＝コード実行のため、**プロジェクト信頼が有効なときだけ実行**（`.constraints.toml`／`.hooks.toml` は設計者が書きエンドユーザーが開くため）。
- **`fail_policy`**: `error`（中止）/ `warn`（警告のみ）/ `ignore` の3種。
- **CWD と変数**: フックの作業ディレクトリは**プロジェクトルート**、`{csv_path}` 等の変数は**絶対パス**で渡す。

### 8.1 プロジェクト設定（`.project.toml`）

- 旧 `config.yaml` の `project`（name / root 等）は **`.project.toml`（TOML）** に置く。
- ファイル分担: `.project.toml`（プロジェクト設定）／ `.constraints.toml`（制約）／ `.hooks.toml`（フック）の3点。

---

## 9. ドメインモデル（構造体とメソッド）

> D-029 に基づく **型駆動設計**。「厳密な構造体＋その振る舞い（メソッド）」を先に確定し、あとは薄いヘルパと Tauri コマンドの配線だけでアプリが繋がる、という方針。以下はシグネチャ主体の設計（実装本体は含めない）。
>
> **エラー方針（D-031 / D-032）**:
> - `Result` の `E` は **`anyhow::Error`**（葉＝アプリのため厳密エラー型は作らない, D-032）。`Result` の Err は「真に処理不能な I/O 等」に限る。
> - **制約違反・設定エラーは"失敗"ではなく"表示するデータ"**（`Vec<Violation>` / `Vec<ConstraintError>`）として返し、**アプリは落とさず必ず開いて UI に表示**する（D-031）。

### 9.1 project — プロジェクト全体

```rust
/// ルートからの相対パスによるシート識別子（例: "modules/auth.csv"）。D-006 / D-011
pub struct SheetId(pub String);

/// プロジェクト全体。ルート配下を1つの編集単位として扱う。
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,        // .project.toml（D-027）
    pub constraints: Constraints,     // .constraints.toml（無ければ空）
    pub hooks: Hooks,                 // .hooks.toml（無ければ空）
    pub git: Option<GitRepo>,         // .git があるときのみ Some（D-007）
    pub trusted: bool,                // 信頼状態（D-027）
    pub constraint_errors: Vec<ConstraintError>, // 設定/制約の不備（表示用, D-031。開けなくはしない）
    sheets: BTreeMap<SheetId, Sheet>, // 相対パス → シート
}

impl Project {
    /// ルートを開き、設定・制約・フック・git を読み込み、CSV を再帰スキャンする。
    /// 制約/設定に不備があっても**失敗させず** constraint_errors に集めて開く（D-031）。
    /// Err は真に処理不能な I/O 等のみ。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self>;
    /// CSV を再帰スキャンしてシート一覧を更新（.git/ と .*.toml は除外）。
    pub fn scan(&mut self) -> Result<()>;
    pub fn sheet_ids(&self) -> Vec<SheetId>;              // 相対パス順
    pub fn sheet(&self, id: &SheetId) -> Option<&Sheet>;
    pub fn sheet_mut(&mut self, id: &SheetId) -> Option<&mut Sheet>;
    pub fn git_enabled(&self) -> bool;                   // = self.git.is_some()（D-007）
    pub fn set_trusted(&mut self, yes: bool);            // フック/コード実行の解禁（D-027）

    /// 保存フロー（7.3 / D-027 / D-031）: CSV を書き込み、内蔵 Constraint 検証と on_save フックを実行。
    /// 違反があっても落とさず、SaveReport に違反・フック結果を載せて返す（UI で表示）。
    pub fn save_sheet(&mut self, id: &SheetId) -> Result<SaveReport>;
    /// エクスポート（on_export フック）。
    pub fn export(&self) -> Vec<HookOutcome>;
}

/// .project.toml の内容（プロジェクト設定, D-027）。
pub struct ProjectConfig { pub name: String, /* 他の設定 */ }
```

### 9.2 sheet — 1枚のシート（CSV）

```rust
pub struct CellPos { pub row: usize, pub col: usize }

/// 表示しない内部用の行ID（D-036）。挿入/削除/並べ替えでも不変。
pub struct RowId(pub u64);

pub struct Sheet {
    pub id: SheetId,
    pub grid: Vec<Vec<String>>,  // 2次元セル（D-003）。先頭行=列ヘッダー
    pub has_row_headers: bool,   // 先頭列を行ラベルとして扱うか（D-013）
    pub style: SheetStyle,       // <name>.style.toml（D-035）
    row_ids: Vec<RowId>,         // grid のデータ行と並行。内部識別（D-036, 非表示）
    next_row_id: u64,            // 採番カウンタ（セッション内一意・再利用なし）
    dirty: bool,
}

impl Sheet {
    pub fn load(root: &Path, id: SheetId, has_row_headers: bool) -> Result<Self>; // csv + style, RowId 採番
    pub fn save(&mut self, root: &Path) -> Result<()>;      // csv + style を書き出し, dirty=false
    pub fn column_headers(&self) -> &[String];              // 先頭行
    pub fn row_headers(&self) -> Option<Vec<&str>>;         // 先頭列（有効時のみ）
    pub fn get(&self, pos: CellPos) -> Option<&str>;
    pub fn set(&mut self, pos: CellPos, value: String);     // dirty=true
    pub fn insert_row(&mut self, at: usize, row: Vec<String>); // 新規 RowId を採番
    pub fn remove_row(&mut self, at: usize);
    pub fn is_dirty(&self) -> bool;
    pub fn row_id(&self, index: usize) -> Option<RowId>;    // index → RowId
    pub fn index_of(&self, id: RowId) -> Option<usize>;     // RowId → 現在の index
    /// 行を「列名→値」の辞書として見る（when 評価・検証で使用）。
    pub fn row_map(&self, row: usize) -> BTreeMap<&str, &str>;
    pub fn set_column_attr(&mut self, column: &str, attr: Attr); // dirty=true
    pub fn set_row_attr(&mut self, row: RowId, attr: Attr);      // dirty=true（内部は RowId, D-036）
}

/// 表示属性（サイドカー `<name>.style.toml`, D-035）。CSV には入れない。
pub struct SheetStyle {
    pub columns: BTreeMap<String, Attr>, // 列名 → 属性
    pub rows: BTreeMap<RowId, Attr>,     // 内部は RowId で保持（D-036, ズレない）
}
/// ディスク上（.style.toml）で行をアンカーする永続キー（保存/ロードで RowId と相互変換, D-036）。
pub enum RowKey { Label(String), Index(usize) }
/// 1つの表示属性セット（当面は行/列単位。将来セル単位も）。
pub struct Attr {
    pub font: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub color: Option<String>,       // 文字色
    pub background: Option<String>,  // 背景色
    // 必要に応じて拡張（配置・幅 等）
}
impl SheetStyle {
    pub fn load(root: &Path, sheet: &SheetId) -> Result<Self>; // 無ければ空
    pub fn save(&self, root: &Path, sheet: &SheetId) -> Result<()>;
}
```

### 9.3 constraints — 制約（第6章の実体）

```rust
pub struct Constraints {
    pub headers: Vec<HeaderConstraint>,
    pub values: Vec<ValueConstraint>,
}

impl Constraints {
    pub fn load(path: &Path) -> Result<Self>;                // .constraints.toml（無ければ空）
    /// 静的検証: 型×規則の非互換(D-021)、header 同士の矛盾(D-012)を検出し**リストで返す**（表示用, D-031）。
    pub fn check_static(&self, sheet_ids: &[SheetId]) -> Vec<ConstraintError>;
    /// あるシートに効く header を AND 合成（D-012）。矛盾は err に載る（落とさない）。
    pub fn effective_header(&self, id: &SheetId) -> (EffectiveHeader<'_>, Vec<ConstraintError>);
    pub fn values_for(&self, id: &SheetId) -> Vec<&ValueConstraint>;      // D-020
    pub fn validate_sheet(&self, sheet: &Sheet) -> Vec<Violation>;
    /// あるセルの enum 候補（プルダウン用）。when を評価して該当する enum を返す。
    pub fn enum_options(&self, sheet: &Sheet, pos: CellPos) -> Option<Vec<String>>;
}

/// header 制約（1シート1つで行・列を同時定義, D-009）。
pub struct HeaderConstraint {
    pub sheet: GlobPattern,
    pub column_require: Vec<String>,       // リテラル厳密一致（D-010）
    pub column_optional: Vec<GlobPattern>, // グロブ可（D-010）
    pub row_require: Vec<String>,
    pub row_optional: Vec<GlobPattern>,
}
impl HeaderConstraint {
    pub fn matches_sheet(&self, id: &SheetId) -> bool;
    pub fn column_allowed(&self, name: &str) -> bool;  // require ∪ optional グロブ（strict 既定 D-010）
    pub fn row_allowed(&self, name: &str) -> bool;
}

/// 複数 header の AND 合成結果（D-012）。
pub struct EffectiveHeader<'a> {
    pub column_required: BTreeSet<String>, // 各 require の和集合
    pub row_required: BTreeSet<String>,
    matched: Vec<&'a HeaderConstraint>,    // 許容判定は全 header の AND
}
impl EffectiveHeader<'_> {
    pub fn column_allowed(&self, name: &str) -> bool;  // matched.iter().all(..)
    pub fn validate(&self, sheet: &Sheet) -> Vec<Violation>;
}

/// value 制約（D-009 / D-017 / D-021）。
pub struct ValueConstraint {
    pub sheet: GlobPattern,
    pub target: Target,           // 列 or 行
    pub rules: ValueRules,
    pub when: Option<WhenExpr>,   // 適用条件（宣言式のみ, D-026）
}
pub enum Target { Column(String), Row(String) }

pub struct ValueRules {
    pub ty: Option<ValueType>,      // string|int|float（D-017）
    pub required: bool,
    pub enums: Option<Vec<String>>, // 単純 enum（D-025）
    pub pattern: Option<String>,    // string のみ（D-021）
    pub min: Option<f64>,           // 数値のみ（D-021）
    pub max: Option<f64>,
}
pub enum ValueType { String, Int, Float }

impl ValueRules {
    pub fn check_compat(&self) -> Result<(), ConstraintError>; // 型×規則の互換性（D-021）
    pub fn check_value(&self, value: &str) -> Result<(), ViolationKind>;
}
impl ValueConstraint {
    pub fn matches_sheet(&self, id: &SheetId) -> bool;
    pub fn applies(&self, sheet: &Sheet, row: usize) -> bool;  // when を評価
    pub fn target_cells(&self, sheet: &Sheet) -> Vec<CellPos>; // Column→列, Row→行
}
```

### 9.4 when 式（宣言式パーサ）

```rust
/// when 式の AST（D-019 / D-026: 宣言式のみ）。
pub enum WhenExpr {
    Or(Vec<WhenExpr>),
    And(Vec<WhenExpr>),
    Cond(Condition),
}
pub struct Condition { pub target: Target, pub op: CompareOp, pub value: CondValue }
pub enum CompareOp { Equals, NotEqual, Gt, Lt, In, Matches } // 6種（D-019）
pub enum CondValue { One(String), Many(Vec<String>) }        // Many は in 用

impl WhenExpr {
    pub fn parse(src: &str) -> Result<Self, ParseError>;         // `{}` 条件, `&`/`|`, `()`
    pub fn eval(&self, row: &BTreeMap<&str, &str>) -> bool;
}
impl Condition { pub fn eval(&self, row: &BTreeMap<&str, &str>) -> bool; }
```

### 9.5 glob — グロブパターン

```rust
/// `*` 等の標準グロブ + `|` による択一（D-022 / D-028）。
pub struct GlobPattern { raw: String, alts: Vec<glob::Pattern> }
impl GlobPattern {
    pub fn parse(raw: &str) -> Result<Self>;   // `|` で分割し複数グロブへ
    pub fn matches(&self, text: &str) -> bool;  // いずれかに一致
}
```

### 9.6 git — Git 操作（CLI 叩き, D-002）

```rust
pub struct GitRepo { root: PathBuf }
pub struct FileStatus { pub path: String, pub state: FileState }
pub enum FileState { Modified, Added, Untracked, Deleted, Staged }
pub struct DiffCell { pub row: usize, pub col: usize, pub old: String, pub new: String, pub status: CellStatus }
pub enum CellStatus { Modified, Added, Removed }
pub struct Commit { pub hash: String, pub message: String, pub author: String, pub timestamp: i64 }

impl GitRepo {
    pub fn discover(root: &Path) -> Option<Self>;                 // .git 探索（D-007）
    pub fn status(&self) -> Result<Vec<FileStatus>>;
    pub fn add(&self, paths: &[SheetId]) -> Result<()>;
    pub fn commit(&self, message: &str) -> Result<Commit>;
    pub fn diff_cells(&self, id: &SheetId) -> Result<Vec<DiffCell>>; // HEAD~1 とのセル差分
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>>;
}
```

### 9.7 hooks — フック（.hooks.toml, D-024 / D-027）

```rust
pub struct Hooks { pub on_save: Vec<Hook>, pub on_export: Vec<Hook> }
pub struct Hook { pub command: String, pub description: String, pub fail_policy: FailPolicy }
pub enum FailPolicy { Error, Warn, Ignore }
pub enum HookEvent { Save, Export }
pub struct HookContext<'a> { pub project_root: &'a Path, pub csv_path: Option<&'a Path>, pub output_path: Option<&'a Path> }
pub enum HookOutcome { Ok, Warned(String), Failed(String) }

impl Hooks {
    pub fn load(path: &Path) -> Result<Self>;
    /// 指定イベントのフックを順に実行。信頼が必要, CWD=root, 変数は絶対パス（D-027）。
    pub fn run(&self, event: HookEvent, ctx: &HookContext, trusted: bool) -> Vec<HookOutcome>;
}
```

### 9.8 validation — 検証結果

```rust
pub struct Violation { pub sheet: SheetId, pub pos: Option<CellPos>, pub kind: ViolationKind, pub message: String }
pub enum ViolationKind {
    MissingRequiredColumn, DisallowedColumn, MissingRequiredRow,
    TypeMismatch, NotInEnum, PatternMismatch, OutOfRange, Empty,
}
```

### 9.9 Tauri コマンド（配線層 = 薄い）

各コマンドは上記メソッドを呼ぶだけの薄いラッパ。IPC 用に serde で（de）シリアライズ。

| コマンド | 呼ぶメソッド |
|---|---|
| `scan_csv_tree` | `Project::scan` → `sheet_ids` |
| `load_csv` | `Sheet::grid`（`Project::sheet`） |
| `save_csv` | `Project::save_sheet`（内蔵検証＋on_save 込み） |
| `validate_sheet` | `Constraints::validate_sheet` |
| `enum_options` | `Constraints::enum_options`（プルダウン） |
| `git_status` / `git_add` / `git_commit` / `git_diff` / `git_log` | `GitRepo::*` |
| `run_export` | `Project::export` |

### 9.10 決定済みの補足

- **エラー型**: `anyhow` を採用（D-032）。制約/検証の不備は Err ではなく表示用データ（`Vec<Violation>` / `Vec<ConstraintError>`）で返す（D-031）。
- **DTO**: 基本はドメイン構造体に `serde` を直接 derive して**共用**。そのまま出せない箇所（ライフタイム付き等）だけ DTO を用意（D-033）。
