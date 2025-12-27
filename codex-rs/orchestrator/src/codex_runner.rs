//! Codexコアを使用した子エージェントランナー
//!
//! 既存のCodexセッション機構を利用して子エージェントを実行する。

use std::time::Instant;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug};

use crate::spec::AgentSpec;
use crate::agent::{
    AgentContext, AgentEvent, AgentResult, AgentStatus,
    ChildAgentRunner, build_dependency_context,
};
use crate::error::OrchestratorError;

/// Codexコアを使用したランナー
///
/// 実際のCodexセッションを使用して子エージェントを実行する。
/// 各子エージェントは独自のサブセッションとして実行される。
pub struct CodexRunner {
    /// 基本設定（モデル名など）
    pub model: String,
    /// 追加の設定
    pub config_overrides: Vec<String>,
}

impl CodexRunner {
    /// 新しいCodexRunnerを作成
    pub fn new(model: String) -> Self {
        Self {
            model,
            config_overrides: Vec::new(),
        }
    }

    /// 設定オーバーライドを追加
    pub fn with_config_override(mut self, key: &str, value: &str) -> Self {
        self.config_overrides.push(format!("{}={}", key, value));
        self
    }
}

#[async_trait]
impl ChildAgentRunner for CodexRunner {
    async fn run(
        &self,
        spec: &AgentSpec,
        context: AgentContext,
        cancel_token: CancellationToken,
    ) -> Result<AgentResult, OrchestratorError> {
        let start = Instant::now();
        let agent_name = spec.name.clone();

        info!("Starting child agent: {}", agent_name);

        // 依存エージェントのコンテキストを構築
        let dependency_context = build_dependency_context(&context.dependency_results);

        // システムプロンプトを構築
        let system_prompt = build_agent_prompt(spec, &context.goal, &dependency_context);

        debug!("Agent {} system prompt:\n{}", agent_name, system_prompt);

        // 進捗イベントを送信
        let _ = context.event_tx.send(AgentEvent::Progress {
            agent_name: agent_name.clone(),
            message: "Initializing agent session...".to_string(),
        }).await;

        // TODO: 実際のCodexセッションを使用
        // 現時点ではプレースホルダー実装
        //
        // 実際の実装では以下のような流れになる:
        // 1. Config::load() で設定を読み込み
        // 2. AuthManager を取得
        // 3. Codex::spawn() でセッションを開始
        // 4. ユーザー入力としてタスクを送信
        // 5. イベントループでツール呼び出しを処理
        // 6. 完了を待機

        let output = execute_with_codex_core(
            spec,
            &system_prompt,
            &context,
            cancel_token.clone(),
        ).await?;

        let elapsed = start.elapsed().as_millis() as u64;

        info!("Child agent {} completed in {}ms", agent_name, elapsed);

        Ok(AgentResult {
            agent_name,
            status: AgentStatus::Completed,
            output,
            files_modified: Vec::new(), // TODO: ファイル変更を追跡
            execution_time_ms: elapsed,
            tokens_used: None, // TODO: トークン使用量を追跡
        })
    }
}

/// エージェント用のプロンプトを構築
fn build_agent_prompt(spec: &AgentSpec, goal: &str, dependency_context: &str) -> String {
    let type_instructions = spec.get_instructions();

    let mut prompt = String::new();

    // プロジェクトゴール
    prompt.push_str(&format!("# Project Goal\n{}\n\n", goal));

    // エージェント固有の指示
    if !type_instructions.is_empty() {
        prompt.push_str(&format!("# Your Role\n{}\n\n", type_instructions.trim()));
    }

    // 依存エージェントのコンテキスト
    if !dependency_context.is_empty() {
        prompt.push_str(&format!(
            "# Context from Previous Agents\n{}\n\n",
            dependency_context
        ));
    }

    // タスク
    prompt.push_str(&format!("# Your Task\n{}\n", spec.task));

    // 追加の制約
    prompt.push_str("\n# Important\n");
    prompt.push_str("- Focus only on your assigned task\n");
    prompt.push_str("- Report any issues or blockers clearly\n");
    prompt.push_str("- Provide clear output that can be used by other agents\n");

    prompt
}

/// Codexコアを使用してタスクを実行
///
/// 現時点ではシミュレーション。実際のCodex統合は今後実装。
async fn execute_with_codex_core(
    spec: &AgentSpec,
    system_prompt: &str,
    context: &AgentContext,
    cancel_token: CancellationToken,
) -> Result<String, OrchestratorError> {
    // TODO: 実際のCodexコア統合
    //
    // 以下のような実装が必要:
    // ```rust
    // use codex_core::{Codex, Config, Event, Op};
    //
    // let config = Config::load_with_overrides(...)?;
    // let (codex, event_rx) = Codex::spawn(config).await?;
    //
    // // システムプロンプトを設定
    // codex.submit(Op::SetInstructions(system_prompt.to_string())).await?;
    //
    // // タスクを送信
    // codex.submit(Op::UserInput(spec.task.clone())).await?;
    //
    // // イベントループ
    // loop {
    //     tokio::select! {
    //         Some(event) = event_rx.recv() => {
    //             match event {
    //                 Event::TurnComplete { output, .. } => {
    //                     return Ok(output);
    //                 }
    //                 Event::ToolCall { name, args, .. } => {
    //                     // ツール呼び出しを転送
    //                     context.event_tx.send(AgentEvent::ToolCall { ... }).await;
    //                 }
    //                 _ => {}
    //             }
    //         }
    //         _ = cancel_token.cancelled() => {
    //             return Err(OrchestratorError::Cancelled);
    //         }
    //     }
    // }
    // ```

    // シミュレーション: 少し待機してから成功を返す
    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
            // 進捗を報告
            let _ = context.event_tx.send(AgentEvent::Progress {
                agent_name: spec.name.clone(),
                message: "Processing task...".to_string(),
            }).await;
        }
        _ = cancel_token.cancelled() => {
            return Err(OrchestratorError::Cancelled);
        }
    }

    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {}
        _ = cancel_token.cancelled() => {
            return Err(OrchestratorError::Cancelled);
        }
    }

    // プレースホルダー出力
    Ok(format!(
        "Agent '{}' completed task: {}\n\n\
         [Note: This is a placeholder. Real Codex integration pending.]\n\n\
         System prompt used:\n{}",
        spec.name,
        spec.task,
        system_prompt
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::spec::AgentType;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_codex_runner() {
        let runner = CodexRunner::new("gpt-4".to_string());

        let spec = AgentSpec {
            name: "test_agent".to_string(),
            agent_type: AgentType::CodeGeneration,
            depends_on: Vec::new(),
            instructions: None,
            task: "Write a hello world function".to_string(),
            config: HashMap::new(),
        };

        let (tx, _rx) = mpsc::channel(100);
        let context = AgentContext {
            goal: "Test project".to_string(),
            dependency_results: Vec::new(),
            event_tx: tx,
            working_dir: std::env::current_dir().unwrap(),
        };

        let result = runner.run(&spec, context, CancellationToken::new()).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.status, AgentStatus::Completed);
    }
}
