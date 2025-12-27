# Multi-Agent Orchestrator

仕様駆動で親エージェントが複数の子エージェントを並列実行するシステム。

## 仕様ファイル形式 (SPEC.md)

```markdown
# Project Specification

## Goal
ユーザー認証機能を実装する

## Agents

### coder
- type: code_generation
- task: |
    src/auth/login.rs にログイン機能を実装
    - ユーザー名とパスワードの検証
    - JWTトークンの発行

### reviewer
- type: review
- depends_on: coder
- task: |
    coder が生成したコードをレビュー
    - セキュリティの確認
    - エラーハンドリングの確認

### tester
- type: test
- depends_on: coder
- task: |
    テストを実装して実行
    - 正常系テスト
    - 異常系テスト

### custom_agent
- type: custom
- instructions: |
    あなたはドキュメント生成の専門家です
- task: |
    APIドキュメントを生成
```

## アーキテクチャ

```
┌─────────────────────────────────────────────────┐
│              Parent Agent (Orchestrator)         │
│  - 仕様をパース                                   │
│  - 子エージェントをスポーン                        │
│  - 結果を集約                                     │
└────────────────────┬────────────────────────────┘
                     │
     ┌───────────────┼───────────────┐
     │               │               │
     ▼               ▼               ▼
┌─────────┐   ┌─────────┐   ┌─────────┐
│ Coder   │   │Reviewer │   │ Tester  │
│ Agent   │   │ Agent   │   │ Agent   │
└─────────┘   └─────────┘   └─────────┘
```

## 子エージェント種類

| Type | 説明 | デフォルト指示 |
|------|------|---------------|
| code_generation | コード生成 | コードを書くことに集中 |
| review | コードレビュー | 問題点を指摘、改善提案 |
| test | テスト実行 | テストを書いて実行 |
| custom | カスタム | instructions で指定 |
