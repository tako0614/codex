//! Multi-Agent Orchestrator for Tacodex
//!
//! 仕様駆動で親エージェントが複数の子エージェントを並列実行するシステム。
//!
//! # 概要
//!
//! このモジュールは、docsフォルダを指定して親エージェントが子エージェントを
//! 管理・スポーンするワークフローを提供します。
//!
//! # 設定 (tacodex.toml)
//!
//! プロジェクトルートに`tacodex.toml`を配置:
//!
//! ```toml
//! # オーケストレーションを有効化
//! enabled = true
//!
//! # ドキュメントフォルダ
//! docs_dir = "./docs"
//!
//! # 並列実行数
//! max_parallel_agents = 8
//!
//! # 子エージェント用モデル
//! child_agent_model = "gpt-4o"
//! ```

pub mod agent;
pub mod auto_setup;
pub mod codex_runner;
pub mod config;
pub mod error;
pub mod orchestrator;
pub mod session;
pub mod spec;
pub mod tacodex_config;

pub use agent::{AgentContext, AgentEvent, AgentResult, AgentStatus, ChildAgent, ChildAgentRunner};
pub use auto_setup::{AutoSetupConfig, AutoSetupManager, build_spec_generation_prompt};
pub use codex_runner::CodexRunner;
pub use config::{OrchestratorConfigToml, ResolvedOrchestratorConfig};
pub use error::OrchestratorError;
pub use orchestrator::{OrchestrationResult, Orchestrator, OrchestratorConfig};
pub use session::{ChildAgentInfo, OrchestratorSessionManager, OrchestratorState, OrchestratorStatusEvent};
pub use spec::{AgentSpec, AgentType, Spec};
pub use tacodex_config::{ResolvedTacodexConfig, TacodexConfig, TacodexConfigError};
