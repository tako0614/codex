//! 親エージェント（オーケストレーター）の実装
//!
//! 複数の子エージェントを並列で管理・実行する。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use futures::future::join_all;
use tracing::{info, warn, debug};

use crate::spec::Spec;
use crate::agent::{
    ChildAgent, AgentContext, AgentEvent, AgentResult, AgentStatus,
    ChildAgentRunner,
};
use crate::error::OrchestratorError;

/// オーケストレーターの設定
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// 作業ディレクトリ
    pub working_dir: PathBuf,
    /// 最大並列数
    pub max_parallelism: usize,
    /// エージェントごとのタイムアウト（秒）
    pub agent_timeout_secs: u64,
    /// 失敗時に続行するか
    pub continue_on_failure: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            max_parallelism: 4,
            agent_timeout_secs: 300,
            continue_on_failure: false,
        }
    }
}

/// オーケストレーション結果
#[derive(Debug)]
pub struct OrchestrationResult {
    /// 全エージェントの結果
    pub results: HashMap<String, AgentResult>,
    /// 成功したエージェント数
    pub succeeded: usize,
    /// 失敗したエージェント数
    pub failed: usize,
    /// 総実行時間（ミリ秒）
    pub total_time_ms: u64,
}

impl OrchestrationResult {
    /// 全体として成功したか
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    /// サマリーを生成
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str(&format!(
            "=== Orchestration Complete ===\n\
             Succeeded: {}\n\
             Failed: {}\n\
             Total Time: {}ms\n\n",
            self.succeeded, self.failed, self.total_time_ms
        ));

        for (name, result) in &self.results {
            summary.push_str(&format!(
                "[{}] {:?} ({}ms)\n",
                name, result.status, result.execution_time_ms
            ));
        }

        summary
    }
}

/// 親エージェント（オーケストレーター）
pub struct Orchestrator<R: ChildAgentRunner> {
    /// 仕様
    spec: Spec,
    /// 設定
    config: OrchestratorConfig,
    /// 子エージェントのマップ
    agents: Arc<RwLock<HashMap<String, Arc<ChildAgent>>>>,
    /// 実行結果
    results: Arc<RwLock<HashMap<String, AgentResult>>>,
    /// エージェント実行用のランナー
    runner: Arc<R>,
    /// キャンセルトークン
    cancel_token: CancellationToken,
    /// イベント送信チャンネル
    event_tx: mpsc::Sender<AgentEvent>,
    /// イベント受信チャンネル
    event_rx: Option<mpsc::Receiver<AgentEvent>>,
}

impl<R: ChildAgentRunner + 'static> Orchestrator<R> {
    /// 新しいオーケストレーターを作成
    pub fn new(spec: Spec, config: OrchestratorConfig, runner: R) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        // 子エージェントを作成
        let agents: HashMap<String, Arc<ChildAgent>> = spec.agents
            .iter()
            .map(|(name, spec)| (name.clone(), Arc::new(ChildAgent::new(spec.clone()))))
            .collect();

        Self {
            spec,
            config,
            agents: Arc::new(RwLock::new(agents)),
            results: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(runner),
            cancel_token: CancellationToken::new(),
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    /// イベント受信チャンネルを取得（一度だけ）
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<AgentEvent>> {
        self.event_rx.take()
    }

    /// オーケストレーションを実行
    pub async fn run(&self) -> Result<OrchestrationResult, OrchestratorError> {
        let start = Instant::now();
        info!("Starting orchestration with {} agents", self.spec.agents.len());

        // 実行順序を取得（依存関係を考慮）
        let execution_order = self.spec.execution_order()?;
        debug!("Execution order: {:?}", execution_order);

        // バッチごとに実行
        for batch in execution_order {
            if self.cancel_token.is_cancelled() {
                return Err(OrchestratorError::Cancelled);
            }

            info!("Executing batch: {:?}", batch);

            // バッチ内のエージェントを並列実行
            let batch_result = self.run_batch(&batch).await?;

            // 失敗があれば、設定に応じて処理
            if !self.config.continue_on_failure {
                for result in batch_result.values() {
                    if matches!(result.status, AgentStatus::Failed(_)) {
                        warn!("Agent {} failed, stopping orchestration", result.agent_name);
                        return Ok(self.build_result(start.elapsed().as_millis() as u64));
                    }
                }
            }
        }

        Ok(self.build_result(start.elapsed().as_millis() as u64))
    }

    /// バッチ内のエージェントを並列実行
    async fn run_batch(&self, agent_names: &[String]) -> Result<HashMap<String, AgentResult>, OrchestratorError> {
        let mut handles = Vec::new();

        // セマフォで並列数を制限
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_parallelism));

        for agent_name in agent_names {
            let agent_name = agent_name.clone();
            let agents = self.agents.clone();
            let results = self.results.clone();
            let runner = self.runner.clone();
            let goal = self.spec.goal.clone();
            let event_tx = self.event_tx.clone();
            let working_dir = self.config.working_dir.clone();
            let cancel_token = self.cancel_token.child_token();
            let timeout = self.config.agent_timeout_secs;
            let semaphore = semaphore.clone();

            let handle = tokio::spawn(async move {
                // セマフォで待機
                let _permit = semaphore.acquire().await.unwrap();

                // エージェントを取得
                let agent = {
                    let agents_guard = agents.read().await;
                    agents_guard.get(&agent_name).cloned()
                };

                let agent = match agent {
                    Some(a) => a,
                    None => {
                        return AgentResult::failure(
                            agent_name.clone(),
                            "Agent not found".to_string(),
                        );
                    }
                };

                // 依存エージェントの結果を収集
                let dependency_results = {
                    let results_guard = results.read().await;
                    agent.spec.depends_on
                        .iter()
                        .filter_map(|dep| results_guard.get(dep).cloned())
                        .collect::<Vec<_>>()
                };

                // 依存の失敗をチェック
                for dep_result in &dependency_results {
                    if matches!(dep_result.status, AgentStatus::Failed(_)) {
                        return AgentResult::failure(
                            agent_name.clone(),
                            format!("Dependency {} failed", dep_result.agent_name),
                        );
                    }
                }

                // 状態を更新
                agent.set_status(AgentStatus::Running).await;

                // イベントを送信
                let _ = event_tx.send(AgentEvent::Started {
                    agent_name: agent_name.clone(),
                }).await;

                // コンテキストを作成
                let context = AgentContext {
                    goal,
                    dependency_results,
                    event_tx: event_tx.clone(),
                    working_dir,
                };

                // タイムアウト付きで実行
                let result = tokio::select! {
                    result = runner.run(&agent.spec, context, cancel_token.clone()) => {
                        match result {
                            Ok(r) => r,
                            Err(e) => AgentResult::failure(agent_name.clone(), e.to_string()),
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(timeout)) => {
                        cancel_token.cancel();
                        AgentResult::failure(agent_name.clone(), "Timeout".to_string())
                    }
                    _ = cancel_token.cancelled() => {
                        AgentResult {
                            agent_name: agent_name.clone(),
                            status: AgentStatus::Cancelled,
                            output: "Cancelled".to_string(),
                            files_modified: Vec::new(),
                            execution_time_ms: 0,
                            tokens_used: None,
                        }
                    }
                };

                // 結果を保存
                agent.set_result(result.clone()).await;
                agent.set_status(result.status.clone()).await;

                // 完了イベントを送信
                let _ = event_tx.send(AgentEvent::Completed {
                    agent_name: agent_name.clone(),
                    result: result.clone(),
                }).await;

                result
            });

            handles.push(handle);
        }

        // 全てのハンドルを待機
        let batch_results: Vec<AgentResult> = join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        // 結果を保存
        let mut results_guard = self.results.write().await;
        let mut result_map = HashMap::new();
        for result in batch_results {
            result_map.insert(result.agent_name.clone(), result.clone());
            results_guard.insert(result.agent_name.clone(), result);
        }

        Ok(result_map)
    }

    /// 結果を構築
    fn build_result(&self, total_time_ms: u64) -> OrchestrationResult {
        let results = self.results.blocking_read();

        let succeeded = results.values()
            .filter(|r| matches!(r.status, AgentStatus::Completed))
            .count();

        let failed = results.values()
            .filter(|r| matches!(r.status, AgentStatus::Failed(_)))
            .count();

        OrchestrationResult {
            results: results.clone(),
            succeeded,
            failed,
            total_time_ms,
        }
    }

    /// キャンセル
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 現在の状態を取得
    pub async fn get_status(&self) -> HashMap<String, AgentStatus> {
        let agents = self.agents.read().await;
        let mut status = HashMap::new();

        for (name, agent) in agents.iter() {
            status.insert(name.clone(), agent.status().await);
        }

        status
    }
}

/// モックランナー（テスト用）
#[cfg(test)]
pub struct MockRunner;

#[cfg(test)]
#[async_trait::async_trait]
impl ChildAgentRunner for MockRunner {
    async fn run(
        &self,
        spec: &crate::spec::AgentSpec,
        _context: AgentContext,
        _cancel_token: CancellationToken,
    ) -> Result<AgentResult, OrchestratorError> {
        // シミュレート用の遅延
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(AgentResult::success(
            spec.name.clone(),
            format!("Mock output for {}", spec.name),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_basic() {
        let markdown = r#"
## Goal
Test goal

## Agents

### agent1
- type: custom
- task: Task 1

### agent2
- type: custom
- depends_on: agent1
- task: Task 2
"#;

        let spec = Spec::from_markdown(markdown).unwrap();
        let config = OrchestratorConfig::default();
        let runner = MockRunner;

        let orchestrator = Orchestrator::new(spec, config, runner);
        let result = orchestrator.run().await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 0);
    }
}
