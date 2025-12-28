//! Codexコアを使用した子エージェントランナー
//!
//! 既存のCodexセッション機構を利用して子エージェントを実行する。

use std::path::PathBuf;
use std::time::Instant;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, error, warn};

use codex_core::AuthManager;
use codex_core::ConversationManager;
use codex_core::config::{Config, ConfigOverrides};
use codex_core::protocol::{AskForApproval, EventMsg, Op, SessionSource};
use codex_protocol::config_types::SandboxMode;
use codex_protocol::user_input::UserInput;

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
    /// コンフィグプロファイル
    pub config_profile: Option<String>,
    /// 追加の設定
    pub config_overrides: Vec<(String, String)>,
    /// Codexホームディレクトリ
    pub codex_home: PathBuf,
}

impl CodexRunner {
    /// 新しいCodexRunnerを作成
    pub fn new(model: String, codex_home: PathBuf) -> Self {
        Self {
            model,
            config_profile: None,
            config_overrides: Vec::new(),
            codex_home,
        }
    }

    /// プロファイルを設定
    pub fn with_profile(mut self, profile: String) -> Self {
        self.config_profile = Some(profile);
        self
    }

    /// 設定オーバーライドを追加
    pub fn with_config_override(mut self, key: &str, value: &str) -> Self {
        self.config_overrides.push((key.to_string(), value.to_string()));
        self
    }

    /// Configを構築
    async fn build_config(&self, working_dir: &PathBuf, base_instructions: &str) -> Result<Config, OrchestratorError> {
        let overrides = ConfigOverrides {
            model: Some(self.model.clone()),
            config_profile: self.config_profile.clone(),
            approval_policy: Some(AskForApproval::Never), // 子エージェントは自動実行
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            cwd: Some(working_dir.clone()),
            base_instructions: Some(base_instructions.to_string()),
            ..ConfigOverrides::default()
        };

        let cli_overrides: Vec<(String, toml::Value)> = self.config_overrides
            .iter()
            .map(|(k, v)| (k.clone(), toml::Value::String(v.clone())))
            .collect();

        Config::load_with_cli_overrides_and_harness_overrides(cli_overrides, overrides)
            .await
            .map_err(|e| OrchestratorError::ConfigError(e.to_string()))
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

        // Configを構築
        let config = self.build_config(&context.working_dir, &system_prompt).await?;

        // AuthManagerを取得
        let auth_manager = AuthManager::shared(
            self.codex_home.clone(),
            true,
            config.cli_auth_credentials_store_mode,
        );

        // ConversationManagerを作成
        let conversation_manager = ConversationManager::new(
            auth_manager.clone(),
            SessionSource::Exec, // 子エージェントはExecモードで実行
        );

        // 新しい会話を開始
        let new_conv = conversation_manager
            .new_conversation(config.clone())
            .await
            .map_err(|e| OrchestratorError::SessionError(e.to_string()))?;

        let conversation = new_conv.conversation;

        // 進捗イベントを送信
        let _ = context.event_tx.send(AgentEvent::Progress {
            agent_name: agent_name.clone(),
            message: "Sending task to agent...".to_string(),
        }).await;

        // タスクを送信
        let user_input = vec![UserInput::Text {
            text: spec.task.clone(),
        }];

        let default_model = conversation_manager
            .get_models_manager()
            .get_model(&config.model, &config)
            .await;

        conversation
            .submit(Op::UserTurn {
                items: user_input,
                cwd: context.working_dir.clone(),
                approval_policy: AskForApproval::Never,
                sandbox_policy: config.sandbox_policy.get().clone(),
                model: default_model,
                effort: config.model_reasoning_effort,
                summary: config.model_reasoning_summary,
                final_output_json_schema: None,
            })
            .await
            .map_err(|e| OrchestratorError::SessionError(e.to_string()))?;

        // イベントループ
        let mut output = String::new();
        let mut files_modified = Vec::new();
        let mut tokens_used: Option<u64> = None;
        let mut error_message: Option<String> = None;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    warn!("Agent {} cancelled", agent_name);
                    conversation.submit(Op::Interrupt).await.ok();
                    return Ok(AgentResult {
                        agent_name,
                        status: AgentStatus::Cancelled,
                        output: "Cancelled".to_string(),
                        files_modified: Vec::new(),
                        execution_time_ms: start.elapsed().as_millis() as u64,
                        tokens_used: None,
                    });
                }
                result = conversation.next_event() => {
                    match result {
                        Ok(event) => {
                            debug!("Agent {} event: {:?}", agent_name, event.msg);

                            match &event.msg {
                                EventMsg::AgentMessage(msg) => {
                                    // エージェントの出力を収集
                                    output.push_str(&msg.message);
                                    output.push('\n');

                                    // 進捗イベントを送信
                                    let _ = context.event_tx.send(AgentEvent::Progress {
                                        agent_name: agent_name.clone(),
                                        message: "Agent responding...".to_string(),
                                    }).await;
                                }
                                EventMsg::ExecCommandBegin(cmd) => {
                                    let _ = context.event_tx.send(AgentEvent::ToolCall {
                                        agent_name: agent_name.clone(),
                                        tool_name: "shell".to_string(),
                                        arguments: cmd.command.join(" "),
                                    }).await;
                                }
                                EventMsg::PatchApplyBegin(patch) => {
                                    // ファイル変更を追跡
                                    for path in patch.changes.keys() {
                                        files_modified.push(path.to_string_lossy().to_string());
                                    }

                                    let _ = context.event_tx.send(AgentEvent::ToolCall {
                                        agent_name: agent_name.clone(),
                                        tool_name: "apply_patch".to_string(),
                                        arguments: format!("{} files", patch.changes.len()),
                                    }).await;
                                }
                                EventMsg::TaskComplete(tc) => {
                                    // タスク完了
                                    info!("Agent {} task complete", agent_name);
                                    if let Some(last_msg) = &tc.last_agent_message {
                                        output = last_msg.clone();
                                    }
                                    break;
                                }
                                EventMsg::Error(err) => {
                                    error!("Agent {} error: {:?}", agent_name, err);
                                    error_message = Some(format!("{:?}", err));
                                }
                                EventMsg::TokenCount(usage) => {
                                    if let Some(info) = &usage.info {
                                        tokens_used = Some(info.total_token_usage.total_tokens as u64);
                                    }
                                }
                                EventMsg::ShutdownComplete => {
                                    info!("Agent {} shutdown complete", agent_name);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            error!("Agent {} event error: {:?}", agent_name, e);
                            error_message = Some(e.to_string());
                            break;
                        }
                    }
                }
            }
        }

        // シャットダウン
        conversation.submit(Op::Shutdown).await.ok();

        let elapsed = start.elapsed().as_millis() as u64;
        info!("Child agent {} completed in {}ms", agent_name, elapsed);

        // 結果を構築
        if let Some(err) = error_message {
            Ok(AgentResult {
                agent_name,
                status: AgentStatus::Failed(err),
                output,
                files_modified,
                execution_time_ms: elapsed,
                tokens_used,
            })
        } else {
            Ok(AgentResult {
                agent_name,
                status: AgentStatus::Completed,
                output,
                files_modified,
                execution_time_ms: elapsed,
                tokens_used,
            })
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::spec::AgentType;

    #[test]
    fn test_build_agent_prompt() {
        let spec = AgentSpec {
            name: "test_agent".to_string(),
            agent_type: AgentType::CodeGeneration,
            depends_on: Vec::new(),
            instructions: None,
            task: "Write a hello world function".to_string(),
            config: HashMap::new(),
        };

        let prompt = build_agent_prompt(&spec, "Test project", "");

        assert!(prompt.contains("# Project Goal"));
        assert!(prompt.contains("Test project"));
        assert!(prompt.contains("# Your Task"));
        assert!(prompt.contains("Write a hello world function"));
    }
}
