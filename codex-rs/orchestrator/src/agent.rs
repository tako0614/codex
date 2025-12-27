//! 子エージェントの定義と管理

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::spec::{AgentSpec, AgentType};
use crate::error::OrchestratorError;

/// エージェントの実行状態
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// 待機中
    Pending,
    /// 依存待ち
    WaitingForDependencies,
    /// 実行中
    Running,
    /// 成功
    Completed,
    /// 失敗
    Failed(String),
    /// キャンセル
    Cancelled,
}

/// エージェントの実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// エージェント名
    pub agent_name: String,
    /// 実行状態
    pub status: AgentStatus,
    /// 出力テキスト
    pub output: String,
    /// 生成・変更したファイル
    pub files_modified: Vec<String>,
    /// 実行時間（ミリ秒）
    pub execution_time_ms: u64,
    /// トークン使用量
    pub tokens_used: Option<u64>,
}

impl AgentResult {
    /// 成功結果を作成
    pub fn success(agent_name: String, output: String) -> Self {
        Self {
            agent_name,
            status: AgentStatus::Completed,
            output,
            files_modified: Vec::new(),
            execution_time_ms: 0,
            tokens_used: None,
        }
    }

    /// 失敗結果を作成
    pub fn failure(agent_name: String, error: String) -> Self {
        Self {
            agent_name: agent_name.clone(),
            status: AgentStatus::Failed(error.clone()),
            output: error,
            files_modified: Vec::new(),
            execution_time_ms: 0,
            tokens_used: None,
        }
    }
}

/// エージェントからのイベント
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// 開始
    Started { agent_name: String },
    /// 進捗
    Progress {
        agent_name: String,
        message: String
    },
    /// ツール実行
    ToolCall {
        agent_name: String,
        tool_name: String,
        arguments: String
    },
    /// ツール結果
    ToolResult {
        agent_name: String,
        tool_name: String,
        result: String
    },
    /// 完了
    Completed {
        agent_name: String,
        result: AgentResult
    },
    /// エラー
    Error {
        agent_name: String,
        error: String
    },
}

/// 子エージェントのトレイト
#[async_trait]
pub trait ChildAgentRunner: Send + Sync {
    /// エージェントを実行
    async fn run(
        &self,
        spec: &AgentSpec,
        context: AgentContext,
        cancel_token: CancellationToken,
    ) -> Result<AgentResult, OrchestratorError>;
}

/// エージェントの実行コンテキスト
#[derive(Clone)]
pub struct AgentContext {
    /// 親のゴール
    pub goal: String,
    /// 依存エージェントの結果
    pub dependency_results: Vec<AgentResult>,
    /// イベント送信チャンネル
    pub event_tx: mpsc::Sender<AgentEvent>,
    /// 作業ディレクトリ
    pub working_dir: std::path::PathBuf,
}

/// 子エージェントの実体
pub struct ChildAgent {
    /// エージェント仕様
    pub spec: AgentSpec,
    /// 実行状態
    status: Arc<RwLock<AgentStatus>>,
    /// 実行結果
    result: Arc<RwLock<Option<AgentResult>>>,
}

impl ChildAgent {
    /// 新しい子エージェントを作成
    pub fn new(spec: AgentSpec) -> Self {
        Self {
            spec,
            status: Arc::new(RwLock::new(AgentStatus::Pending)),
            result: Arc::new(RwLock::new(None)),
        }
    }

    /// 状態を取得
    pub async fn status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    /// 状態を設定
    pub async fn set_status(&self, status: AgentStatus) {
        *self.status.write().await = status;
    }

    /// 結果を取得
    pub async fn result(&self) -> Option<AgentResult> {
        self.result.read().await.clone()
    }

    /// 結果を設定
    pub async fn set_result(&self, result: AgentResult) {
        *self.result.write().await = Some(result);
    }

    /// エージェントの名前を取得
    pub fn name(&self) -> &str {
        &self.spec.name
    }

    /// エージェントの種類を取得
    pub fn agent_type(&self) -> &AgentType {
        &self.spec.agent_type
    }

    /// システムプロンプトを生成
    pub fn build_system_prompt(&self, goal: &str, dependency_context: &str) -> String {
        let type_instructions = self.spec.get_instructions();

        let mut prompt = String::new();

        // プロジェクトゴール
        prompt.push_str(&format!("# Project Goal\n{}\n\n", goal));

        // エージェント固有の指示
        if !type_instructions.is_empty() {
            prompt.push_str(&format!("# Your Role\n{}\n\n", type_instructions.trim()));
        }

        // 依存エージェントのコンテキスト
        if !dependency_context.is_empty() {
            prompt.push_str(&format!("# Context from Previous Agents\n{}\n\n", dependency_context));
        }

        // タスク
        prompt.push_str(&format!("# Your Task\n{}\n", self.spec.task));

        prompt
    }
}

/// 依存エージェントの結果からコンテキストを生成
pub fn build_dependency_context(results: &[AgentResult]) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut context = String::new();

    for result in results {
        context.push_str(&format!("## {} の結果\n", result.agent_name));
        context.push_str(&format!("状態: {:?}\n", result.status));

        if !result.output.is_empty() {
            context.push_str(&format!("出力:\n{}\n", result.output));
        }

        if !result.files_modified.is_empty() {
            context.push_str("変更されたファイル:\n");
            for file in &result.files_modified {
                context.push_str(&format!("- {}\n", file));
            }
        }

        context.push('\n');
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_result_success() {
        let result = AgentResult::success(
            "test_agent".to_string(),
            "Task completed".to_string(),
        );
        assert_eq!(result.status, AgentStatus::Completed);
    }

    #[test]
    fn test_build_system_prompt() {
        let spec = AgentSpec {
            name: "coder".to_string(),
            agent_type: AgentType::CodeGeneration,
            depends_on: Vec::new(),
            instructions: None,
            task: "Implement login function".to_string(),
            config: std::collections::HashMap::new(),
        };

        let agent = ChildAgent::new(spec);
        let prompt = agent.build_system_prompt("Build auth system", "");

        assert!(prompt.contains("Build auth system"));
        assert!(prompt.contains("Implement login function"));
    }
}
