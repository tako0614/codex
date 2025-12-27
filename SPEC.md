# サンプルプロジェクト仕様

## Goal

シンプルなTodoアプリケーションのバックエンドAPIを実装する

## Agents

### api_designer
- type: custom
- instructions: |
    あなたはAPI設計の専門家です。
    RESTful APIの設計原則に従ってください。
- task: |
    Todo APIのエンドポイント設計を行う
    - GET /todos - 一覧取得
    - POST /todos - 新規作成
    - PUT /todos/:id - 更新
    - DELETE /todos/:id - 削除

### coder
- type: code_generation
- depends_on: api_designer
- task: |
    api_designerの設計に基づいて、
    src/api/todos.rsにTodo APIを実装する

### tester
- type: test
- depends_on: coder
- task: |
    coderが実装したAPIに対するテストを書く
    - 各エンドポイントの正常系テスト
    - エラーハンドリングのテスト

### reviewer
- type: review
- depends_on: coder, tester
- task: |
    実装とテストコードをレビュー
    - コードの品質確認
    - セキュリティの確認
    - パフォーマンスの確認
