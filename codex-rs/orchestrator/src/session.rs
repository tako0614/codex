//! オーケストレーターセッション管理
//!
//! メインのCodexセッションと統合し、子エージェントの管理を行う。

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock, watch};
use tokio_util::sync::CancellationToken;

use crate::agent::{AgentResult, AgentStatus};
use crate::config::ResolvedOrchestratorConfig;
use crate::error::OrchestratorError;
use crate::spec::Spec;

/// 子エージェントのステータス情報（TUI表示用）
#[derive(Debug, Clone)]
pub struct ChildAgentInfo {
    /// エージェント名
    pub name: String,
    /// 現在のステータス
    pub status: AgentStatus,
    /// タスクの説明
    pub task: String,
    /// 進捗メッセージ
    pub progress_message: Option<String>,
    /// 実行時間（ミリ秒）
    pub elapsed_ms: u64,
}

/// オーケストレーターセッションの状態
#[derive(Debug, Clone)]
pub struct OrchestratorState {
    /// オーケストレーションが有効か
    pub enabled: bool,
    /// 現在アクティブか
    pub active: bool,
    /// docsフォルダのパス
    pub docs_dir: Option<PathBuf>,
    /// 子エージェントの情報
    pub child_agents: Vec<ChildAgentInfo>,
    /// 完了したエージェント数
    pub completed_count: usize,
    /// 失敗したエージェント数
    pub failed_count: usize,
    /// 全体の進捗メッセージ
    pub overall_message: String,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            docs_dir: None,
            child_agents: Vec::new(),
            completed_count: 0,
            failed_count: 0,
            overall_message: String::new(),
        }
    }
}

/// TUI用のステータス更新イベント
#[derive(Debug, Clone)]
pub enum OrchestratorStatusEvent {
    /// 初期化完了
    Initialized { docs_dir: PathBuf },
    /// オーケストレーション開始
    Started { total_agents: usize },
    /// エージェント追加
    AgentAdded { info: ChildAgentInfo },
    /// エージェントステータス更新
    AgentUpdated { name: String, status: AgentStatus, message: Option<String> },
    /// エージェント完了
    AgentCompleted { name: String, result: AgentResult },
    /// 全体完了
    Completed { succeeded: usize, failed: usize },
    /// エラー
    Error { message: String },
}

/// オーケストレーターセッションマネージャー
///
/// メインCodexセッションから使用され、子エージェントのライフサイクルを管理する。
pub struct OrchestratorSessionManager {
    /// 設定
    config: ResolvedOrchestratorConfig,
    /// 現在の状態
    state: Arc<RwLock<OrchestratorState>>,
    /// 状態変更の通知チャンネル
    state_tx: watch::Sender<OrchestratorState>,
    /// 状態変更の受信（クローン可能）
    state_rx: watch::Receiver<OrchestratorState>,
    /// イベント送信
    event_tx: mpsc::Sender<OrchestratorStatusEvent>,
    /// イベント受信（オプション）
    event_rx: Option<mpsc::Receiver<OrchestratorStatusEvent>>,
    /// キャンセルトークン
    cancel_token: CancellationToken,
}

impl OrchestratorSessionManager {
    /// 新しいセッションマネージャーを作成
    pub fn new(config: ResolvedOrchestratorConfig) -> Self {
        let initial_state = OrchestratorState {
            enabled: config.enabled,
            docs_dir: config.docs_dir.clone(),
            ..Default::default()
        };

        let (state_tx, state_rx) = watch::channel(initial_state.clone());
        let (event_tx, event_rx) = mpsc::channel(100);

        Self {
            config,
            state: Arc::new(RwLock::new(initial_state)),
            state_tx,
            state_rx,
            event_tx,
            event_rx: Some(event_rx),
            cancel_token: CancellationToken::new(),
        }
    }

    /// イベント受信チャンネルを取得（一度だけ）
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<OrchestratorStatusEvent>> {
        self.event_rx.take()
    }

    /// 状態監視用のReceiverを取得
    pub fn state_receiver(&self) -> watch::Receiver<OrchestratorState> {
        self.state_rx.clone()
    }

    /// 現在の状態を取得
    pub async fn get_state(&self) -> OrchestratorState {
        self.state.read().await.clone()
    }

    /// オーケストレーションを有効化
    pub async fn enable(&self, docs_dir: PathBuf) -> Result<(), OrchestratorError> {
        let mut state = self.state.write().await;
        state.enabled = true;
        state.docs_dir = Some(docs_dir.clone());
        state.overall_message = format!("Docs: {}", docs_dir.display());

        let _ = self.state_tx.send(state.clone());
        let _ = self.event_tx.send(OrchestratorStatusEvent::Initialized { docs_dir }).await;

        Ok(())
    }

    /// docsフォルダからスペックを読み込み
    pub async fn load_specs(&self) -> Result<Vec<Spec>, OrchestratorError> {
        let state = self.state.read().await;
        let docs_dir = state.docs_dir.as_ref()
            .ok_or(OrchestratorError::ConfigError("docs_dir not set".to_string()))?;

        let mut specs = Vec::new();

        // docs/内のmarkdownファイルを探索
        if docs_dir.exists() {
            for entry in std::fs::read_dir(docs_dir)
                .map_err(|e| OrchestratorError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| OrchestratorError::IoError(e.to_string()))?;
                let path = entry.path();

                if path.extension().map_or(false, |ext| ext == "md") {
                    // SPEC.md または *_spec.md パターンをチェック
                    let filename = path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    if filename == "SPEC.md"
                        || filename.ends_with("_spec.md")
                        || filename.ends_with("_SPEC.md")
                    {
                        match Spec::from_file(&path) {
                            Ok(spec) => specs.push(spec),
                            Err(e) => {
                                tracing::warn!("Failed to parse spec file {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(specs)
    }

    /// 子エージェントを追加
    pub async fn add_child_agent(&self, name: String, task: String) {
        let info = ChildAgentInfo {
            name: name.clone(),
            status: AgentStatus::Pending,
            task,
            progress_message: None,
            elapsed_ms: 0,
        };

        {
            let mut state = self.state.write().await;
            state.child_agents.push(info.clone());
            let _ = self.state_tx.send(state.clone());
        }

        let _ = self.event_tx.send(OrchestratorStatusEvent::AgentAdded { info }).await;
    }

    /// 子エージェントのステータスを更新
    pub async fn update_child_status(&self, name: &str, status: AgentStatus, message: Option<String>) {
        {
            let mut state = self.state.write().await;
            if let Some(agent) = state.child_agents.iter_mut().find(|a| a.name == name) {
                agent.status = status.clone();
                agent.progress_message = message.clone();

                match &status {
                    AgentStatus::Completed => state.completed_count += 1,
                    AgentStatus::Failed(_) => state.failed_count += 1,
                    _ => {}
                }
            }
            let _ = self.state_tx.send(state.clone());
        }

        let _ = self.event_tx.send(OrchestratorStatusEvent::AgentUpdated {
            name: name.to_string(),
            status,
            message,
        }).await;
    }

    /// オーケストレーションを開始
    pub async fn start(&self) -> Result<(), OrchestratorError> {
        let mut state = self.state.write().await;
        state.active = true;
        state.overall_message = "Orchestration started".to_string();
        let total = state.child_agents.len();
        let _ = self.state_tx.send(state.clone());

        let _ = self.event_tx.send(OrchestratorStatusEvent::Started { total_agents: total }).await;
        Ok(())
    }

    /// オーケストレーションを停止
    pub async fn stop(&self) {
        self.cancel_token.cancel();
        let mut state = self.state.write().await;
        state.active = false;
        let _ = self.state_tx.send(state.clone());
    }

    /// キャンセルトークンを取得
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// 設定を取得
    pub fn config(&self) -> &ResolvedOrchestratorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager() {
        let config = ResolvedOrchestratorConfig {
            enabled: true,
            docs_dir: Some(PathBuf::from("./docs")),
            ..Default::default()
        };

        let manager = OrchestratorSessionManager::new(config);

        let state = manager.get_state().await;
        assert!(state.enabled);
        assert!(!state.active);
    }

    #[tokio::test]
    async fn test_add_child_agent() {
        let config = ResolvedOrchestratorConfig::default();
        let manager = OrchestratorSessionManager::new(config);

        manager.add_child_agent("test_agent".to_string(), "Test task".to_string()).await;

        let state = manager.get_state().await;
        assert_eq!(state.child_agents.len(), 1);
        assert_eq!(state.child_agents[0].name, "test_agent");
    }
}
