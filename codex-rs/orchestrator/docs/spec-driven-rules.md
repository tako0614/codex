# 仕様駆動開発ルール設計

## 問題点

現状はエージェント任せで以下の問題がある:
- いつ仕様モードに入るかが不明確
- 仕様の構造が一貫しない
- 検証プロセスがない

## 提案: システム側でのルール制御

### 1. 自動検出ルール

**仕様駆動モードをトリガーする条件:**

```
IF (以下のいずれかに該当):
  - ユーザーリクエストに「作って」「構築」「実装して」「開発して」が含まれる
  - AND 対象が複数コンポーネント（frontend + backend, API + UI 等）
  - AND 現在のディレクトリが空または新規プロジェクト

THEN:
  → 仕様駆動モードを有効化
  → まずSPEC.mdを生成してからユーザーに確認
```

**単一エージェントモードを維持する条件:**
```
IF:
  - 単一ファイルの修正
  - バグ修正
  - 既存コードへの機能追加
  - 質問への回答

THEN:
  → 通常の単一エージェントモード
```

### 2. 強制フロー

```
┌─────────────────────────────────────────────────┐
│ ユーザーリクエスト受信                           │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│ [SYSTEM] リクエスト分類                          │
│                                                 │
│ 分類ルール (LLM判定ではなくルールベース):         │
│ - キーワード検出: 作る/構築/実装/開発            │
│ - スコープ検出: 複数コンポーネント言及           │
│ - コンテキスト: 空ディレクトリ/新規プロジェクト   │
└─────────────────────────────────────────────────┘
                    ↓
        ┌─────────┴─────────┐
        ↓                   ↓
   [単一タスク]         [複合タスク]
        ↓                   ↓
   通常実行            仕様駆動モード
                            ↓
                ┌───────────────────────┐
                │ Phase 1: 仕様生成      │
                │ - .tacodex/ 作成      │
                │ - SPEC.md 自動生成    │
                │ - ユーザー確認待ち    │
                └───────────────────────┘
                            ↓
                ┌───────────────────────┐
                │ Phase 2: 仕様承認      │
                │ - ユーザーがSPECを確認 │
                │ - 修正要求 or 承認     │
                └───────────────────────┘
                            ↓
                ┌───────────────────────┐
                │ Phase 3: 実行         │
                │ - tacodex spec 実行   │
                │ - マルチエージェント   │
                └───────────────────────┘
```

### 3. SPEC.md 標準フォーマット (強制)

```markdown
# Project: {project_name}

## Goal
{1-2文でゴールを記述 - 必須}

## Tech Stack
- Backend: {framework}
- Frontend: {framework}
- Database: {database}
- Other: {その他}

## Structure
{生成するフォルダ/ファイル構造}

## Agents

### phase1_architect
- type: architect
- task: |
    {具体的なタスク}
- outputs:
    - package.json
    - folder structure
    - README.md

### phase2_backend
- type: backend
- depends_on: phase1_architect
- task: |
    {具体的なタスク}
- outputs:
    - src/server/**

### phase2_frontend
- type: frontend
- depends_on: phase1_architect
- task: |
    {具体的なタスク}
- outputs:
    - src/client/**

### phase3_integration
- type: integration
- depends_on: phase2_backend, phase2_frontend
- task: |
    {具体的なタスク}

### phase4_testing
- type: testing
- depends_on: phase3_integration
- task: |
    {具体的なタスク}
```

### 4. 検証ルール

**SPEC.md バリデーション:**
```
必須フィールド:
  - Goal (空でない)
  - Tech Stack (1つ以上)
  - Agents (1つ以上)

エージェント検証:
  - 各エージェントに type と task が必須
  - depends_on の循環依存チェック
  - phase番号の整合性チェック

警告:
  - testing エージェントがない場合
  - outputs が未定義の場合
```

### 5. 実装方針

**Option A: CLIレベルで制御**
```bash
# 自動判定して仕様モードへ
tacodex "HonoとViteでSNS作って"
  → 自動的に仕様駆動モードを検出
  → SPEC.md生成 → 確認 → 実行

# 明示的に仕様モード
tacodex --spec "HonoとViteでSNS作って"

# 強制的に単一エージェント
tacodex --no-spec "HonoとViteでSNS作って"
```

**Option B: 対話的フロー**
```
User: HonoとViteでSNS作って

System: このタスクは複数のコンポーネントを含むため、
        仕様駆動モードで進めます。

        まず仕様書(SPEC.md)を作成します...

        [SPEC.md内容を表示]

        この仕様で進めてよろしいですか？
        - [Y] 進める
        - [E] 編集する
        - [N] 単一エージェントで進める

User: Y

System: 仕様に基づいて実装を開始します...
        [Phase 1] architect を実行中...
```

### 6. 設定ファイル

`tacodex.toml` での制御:

```toml
[spec_driven]
# 自動検出を有効化
auto_detect = true

# トリガーキーワード
trigger_keywords = ["作って", "構築", "実装", "開発", "create", "build"]

# 最小コンポーネント数 (これ以上で仕様モード)
min_components = 2

# 確認をスキップ (CI用)
skip_confirmation = false

# デフォルトフェーズ
default_phases = ["architect", "backend", "frontend", "integration", "testing"]
```

## 次のアクション

1. リクエスト分類器の実装 (ルールベース)
2. 強制フローの実装
3. SPEC.mdバリデーターの実装
4. 対話的確認UIの実装
