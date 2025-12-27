//! オーケストレーター設定
//!
//! config.tomlの[orchestrator]セクションを処理する。
//!
//! 設定型は`codex_core::config::types::OrchestratorConfig`で定義されています。
//! このモジュールではランタイム用の解決済み設定を提供します。

use std::path::PathBuf;

// Re-export from codex-core for convenience
pub use codex_core::config::types::OrchestratorConfig as OrchestratorConfigToml;

/// 解決済みのオーケストレーター設定
#[derive(Debug, Clone)]
pub struct ResolvedOrchestratorConfig {
    /// ドキュメントフォルダのパス
    pub docs_dir: Option<PathBuf>,

    /// オーケストレーションモードが有効か
    pub enabled: bool,

    /// 最大並列エージェント数
    pub max_parallel_agents: usize,

    /// エージェントごとのタイムアウト（秒）
    pub agent_timeout_sec: u64,

    /// エラー時に続行するか
    pub continue_on_failure: bool,

    /// 自動的に子エージェントをスポーンするか
    pub auto_spawn: bool,

    /// 子エージェント用のモデル
    pub child_agent_model: Option<String>,
}

impl Default for ResolvedOrchestratorConfig {
    fn default() -> Self {
        Self {
            docs_dir: None,
            enabled: false,
            max_parallel_agents: 4,
            agent_timeout_sec: 300,
            continue_on_failure: false,
            auto_spawn: false,
            child_agent_model: None,
        }
    }
}

impl ResolvedOrchestratorConfig {
    /// ConfigTomlから解決済み設定を生成
    pub fn from_toml(toml: Option<&OrchestratorConfigToml>) -> Self {
        let defaults = Self::default();

        match toml {
            Some(t) => Self {
                docs_dir: t.docs_dir.clone(),
                enabled: t.enabled.unwrap_or(defaults.enabled),
                max_parallel_agents: t.max_parallel_agents.unwrap_or(defaults.max_parallel_agents),
                agent_timeout_sec: t.agent_timeout_sec.unwrap_or(defaults.agent_timeout_sec),
                continue_on_failure: t.continue_on_failure.unwrap_or(defaults.continue_on_failure),
                auto_spawn: t.auto_spawn.unwrap_or(defaults.auto_spawn),
                child_agent_model: t.child_agent_model.clone(),
            },
            None => defaults,
        }
    }

    /// docs_dirが有効な状態かチェック
    pub fn is_docs_available(&self) -> bool {
        if let Some(ref dir) = self.docs_dir {
            dir.exists() && dir.is_dir()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ResolvedOrchestratorConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_parallel_agents, 4);
    }

    #[test]
    fn test_from_toml() {
        let toml = OrchestratorConfigToml {
            docs_dir: Some(PathBuf::from("./docs")),
            enabled: Some(true),
            max_parallel_agents: Some(8),
            ..Default::default()
        };

        let config = ResolvedOrchestratorConfig::from_toml(Some(&toml));
        assert!(config.enabled);
        assert_eq!(config.max_parallel_agents, 8);
    }
}
