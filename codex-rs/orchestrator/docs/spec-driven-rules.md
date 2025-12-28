# 親子エージェントアーキテクチャ

## 概要

Tacodexは親子エージェント構造で動作します:

- **親エージェント（オーケストレーター）**: 計画・仕様作成のみ、コードは書かない
- **子エージェント**: 実際のコーディングを担当

## フロー

```
┌─────────────────────────────────────────────────┐
│ ユーザーリクエスト                               │
│ 例: "HonoとViteでSNS作って"                     │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│ 親エージェント（オーケストレーター）              │
│                                                 │
│ 1. タスク分析                                   │
│ 2. .tacodex/ フォルダ作成                       │
│    ├── SPEC.md         # エージェント定義       │
│    ├── PLAN.md         # 全体計画              │
│    └── instructions/   # 各エージェントへの指示 │
│ 3. ユーザーに案内: "tacodex spec を実行"        │
│                                                 │
│ ※ ソースコードは一切書かない                    │
└─────────────────────────────────────────────────┘
                    ↓
                    ↓
           （自動的に子エージェントを起動）
                    ↓
┌─────────────────────────────────────────────────┐
│ 子エージェント群                                 │
│                                                 │
│ - architect: プロジェクト構造設計               │
│ - backend: API実装                             │
│ - frontend: UI実装                             │
│ - testing: テスト作成                          │
│                                                 │
│ ※ 実際のコードを書く                           │
└─────────────────────────────────────────────────┘
```

## SPEC.md フォーマット

```yaml
# Project: {プロジェクト名}

goal: |
  {目標を1-2文で}

tech_stack:
  backend: Hono
  frontend: Vite + React
  database: SQLite

agents:
  architect:
    type: architect
    instruction_file: .tacodex/instructions/architect.md
    task: |
      プロジェクト構造を設計

  backend:
    type: backend
    depends_on: [architect]
    instruction_file: .tacodex/instructions/backend.md
    task: |
      APIを実装

  frontend:
    type: frontend
    depends_on: [architect]
    instruction_file: .tacodex/instructions/frontend.md
    task: |
      UIを実装

  testing:
    type: testing
    depends_on: [backend, frontend]
    instruction_file: .tacodex/instructions/testing.md
    task: |
      テストを作成・実行
```

## 親エージェントのルール

### やること
- 計画を立てる
- `.tacodex/SPEC.md` を作成
- `.tacodex/instructions/*.md` を作成
- ドキュメントを管理

### やらないこと（絶対禁止）
- ソースコードを書く（.js, .ts, .py, .rs 等）
- ソースファイルを作成・編集する
- テストやビルドを実行する
- npm/cargo/pip などのコマンドを実行する

**注意:** `.tacodex/` 内のドキュメントは `apply_patch` で作成OK
