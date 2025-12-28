//! Auto-Setup Module for Tacodex
//!
//! configがない場合に`.tacodex`フォルダを作成し、
//! ユーザーリクエストから仕様を自動生成する機能を提供。

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::OrchestratorError;

/// `.tacodex` フォルダ内のファイル構造
pub const TACODEX_DIR: &str = ".tacodex";
pub const SPEC_FILE: &str = "SPEC.md";
pub const CONFIG_FILE: &str = "config.toml";
pub const PROMPTS_DIR: &str = "prompts";
pub const CONTEXT_DIR: &str = "context";
pub const AGENTS_DIR: &str = "agents";

/// Auto-Setup設定
#[derive(Debug, Clone)]
pub struct AutoSetupConfig {
    /// 仕様生成に使用するモデル
    pub spec_generation_model: String,
    /// インタラクティブモード（ユーザー確認を求めるか）
    pub interactive: bool,
    /// 最大並列エージェント数
    pub max_parallel_agents: usize,
    /// エージェントタイムアウト（秒）
    pub agent_timeout_sec: u64,
}

impl Default for AutoSetupConfig {
    fn default() -> Self {
        Self {
            spec_generation_model: "gpt-4o".to_string(),
            interactive: true,
            max_parallel_agents: 4,
            agent_timeout_sec: 600,
        }
    }
}

/// Auto-Setupマネージャー
pub struct AutoSetupManager {
    /// プロジェクトルート
    project_root: PathBuf,
    /// `.tacodex`フォルダのパス
    tacodex_dir: PathBuf,
    /// 設定
    config: AutoSetupConfig,
}

impl AutoSetupManager {
    /// 新しいAutoSetupManagerを作成
    pub fn new(project_root: impl AsRef<Path>, config: AutoSetupConfig) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let tacodex_dir = project_root.join(TACODEX_DIR);
        Self {
            project_root,
            tacodex_dir,
            config,
        }
    }

    /// `.tacodex`フォルダが存在するかチェック
    pub fn is_initialized(&self) -> bool {
        self.tacodex_dir.exists() && self.tacodex_dir.join(SPEC_FILE).exists()
    }

    /// configファイルが存在するかチェック
    pub fn has_config(&self) -> bool {
        self.project_root.join("tacodex.toml").exists()
            || self.tacodex_dir.join(CONFIG_FILE).exists()
    }

    /// `.tacodex`フォルダを初期化
    pub fn initialize(&self) -> Result<(), OrchestratorError> {
        // メインフォルダ作成
        fs::create_dir_all(&self.tacodex_dir).map_err(|e| {
            OrchestratorError::IoError(format!(
                "Failed to create .tacodex directory: {}",
                e
            ))
        })?;

        // サブフォルダ作成
        for dir in [PROMPTS_DIR, CONTEXT_DIR, AGENTS_DIR] {
            let path = self.tacodex_dir.join(dir);
            fs::create_dir_all(&path).map_err(|e| {
                OrchestratorError::IoError(format!("Failed to create {} directory: {}", dir, e))
            })?;
        }

        // デフォルトconfig.tomlを作成
        self.create_default_config()?;

        Ok(())
    }

    /// デフォルトのconfig.tomlを作成
    fn create_default_config(&self) -> Result<(), OrchestratorError> {
        let config_path = self.tacodex_dir.join(CONFIG_FILE);
        if config_path.exists() {
            return Ok(());
        }

        let config_content = format!(
            r#"# Tacodex Auto-Setup Configuration
# Generated automatically

[orchestrator]
enabled = true
max_parallel_agents = {}
agent_timeout_sec = {}
continue_on_failure = false
auto_spawn = true
child_agent_model = "{}"

[auto_setup]
enabled = true
spec_generation_model = "{}"
interactive = {}
"#,
            self.config.max_parallel_agents,
            self.config.agent_timeout_sec,
            self.config.spec_generation_model,
            self.config.spec_generation_model,
            self.config.interactive,
        );

        fs::write(&config_path, config_content).map_err(|e| {
            OrchestratorError::IoError(format!("Failed to write config.toml: {}", e))
        })?;

        Ok(())
    }

    /// SPEC.mdのパスを取得
    pub fn spec_path(&self) -> PathBuf {
        self.tacodex_dir.join(SPEC_FILE)
    }

    /// SPEC.mdを保存
    pub fn save_spec(&self, content: &str) -> Result<PathBuf, OrchestratorError> {
        let spec_path = self.spec_path();
        fs::write(&spec_path, content).map_err(|e| {
            OrchestratorError::IoError(format!("Failed to write SPEC.md: {}", e))
        })?;
        Ok(spec_path)
    }

    /// SPEC.mdを読み込み
    pub fn load_spec(&self) -> Result<String, OrchestratorError> {
        let spec_path = self.spec_path();
        fs::read_to_string(&spec_path).map_err(|e| {
            OrchestratorError::IoError(format!("Failed to read SPEC.md: {}", e))
        })
    }

    /// 仕様生成用のシステムプロンプトを取得
    pub fn get_spec_generator_prompt(&self) -> String {
        include_str!("../prompts/spec_generator.md").to_string()
    }

    /// エージェントタイプ別のプロンプトを取得
    pub fn get_agent_prompt(&self, agent_type: &str) -> Option<String> {
        match agent_type {
            "architect" => Some(include_str!("../prompts/architect.md").to_string()),
            "backend" => Some(include_str!("../prompts/backend.md").to_string()),
            "frontend" => Some(include_str!("../prompts/frontend.md").to_string()),
            "testing" => Some(include_str!("../prompts/testing.md").to_string()),
            "review" => Some(include_str!("../prompts/review.md").to_string()),
            _ => None,
        }
    }

    /// プロジェクトルートを取得
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// .tacodexディレクトリを取得
    pub fn tacodex_dir(&self) -> &Path {
        &self.tacodex_dir
    }

    /// エージェント出力ディレクトリを取得
    pub fn agent_output_dir(&self, agent_name: &str) -> PathBuf {
        self.tacodex_dir.join(AGENTS_DIR).join(agent_name)
    }

    /// コンテキストディレクトリを取得
    pub fn context_dir(&self) -> PathBuf {
        self.tacodex_dir.join(CONTEXT_DIR)
    }
}

/// ユーザーリクエストから仕様生成用のプロンプトを構築
pub fn build_spec_generation_prompt(user_request: &str) -> String {
    format!(
        r#"# User Request

{}

# Instructions

上記のユーザーリクエストを分析し、SPEC.mdファイルを生成してください。

必ず以下の手順に従ってください:

1. リクエストを分析し、必要な機能を特定
2. 適切なエージェント（architect, backend, frontend, testing, review等）を定義
3. 依存関係を設定（例: backendはarchitectに依存）
4. 具体的なタスクを各エージェントに割り当て
5. `.tacodex/SPEC.md` にファイルを作成

apply_patchツールを使用してSPEC.mdファイルを作成してください。
"#,
        user_request
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_auto_setup_manager_initialization() {
        let temp_dir = tempdir().unwrap();
        let manager = AutoSetupManager::new(temp_dir.path(), AutoSetupConfig::default());

        assert!(!manager.is_initialized());
        assert!(!manager.has_config());

        manager.initialize().unwrap();

        assert!(manager.tacodex_dir().exists());
        assert!(manager.tacodex_dir().join(CONFIG_FILE).exists());
        assert!(manager.tacodex_dir().join(PROMPTS_DIR).exists());
        assert!(manager.tacodex_dir().join(CONTEXT_DIR).exists());
        assert!(manager.tacodex_dir().join(AGENTS_DIR).exists());
    }

    #[test]
    fn test_save_and_load_spec() {
        let temp_dir = tempdir().unwrap();
        let manager = AutoSetupManager::new(temp_dir.path(), AutoSetupConfig::default());
        manager.initialize().unwrap();

        let spec_content = "# Test Spec\n\n## Goal\nTest goal";
        manager.save_spec(spec_content).unwrap();

        assert!(manager.is_initialized());

        let loaded = manager.load_spec().unwrap();
        assert_eq!(loaded, spec_content);
    }

    #[test]
    fn test_get_agent_prompts() {
        let temp_dir = tempdir().unwrap();
        let manager = AutoSetupManager::new(temp_dir.path(), AutoSetupConfig::default());

        assert!(manager.get_agent_prompt("architect").is_some());
        assert!(manager.get_agent_prompt("backend").is_some());
        assert!(manager.get_agent_prompt("frontend").is_some());
        assert!(manager.get_agent_prompt("testing").is_some());
        assert!(manager.get_agent_prompt("review").is_some());
        assert!(manager.get_agent_prompt("unknown").is_none());
    }
}
