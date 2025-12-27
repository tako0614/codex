//! tacodex.toml設定ファイルの読み込み
//!
//! プロジェクトルートから`tacodex.toml`を探して読み込む。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// tacodex.toml の設定構造
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TacodexConfig {
    /// ドキュメントフォルダのパス（プロジェクトルートからの相対パス）
    #[serde(default)]
    pub docs_dir: Option<PathBuf>,

    /// オーケストレーションモードを有効にするか
    #[serde(default)]
    pub enabled: Option<bool>,

    /// 最大並列エージェント数
    #[serde(default)]
    pub max_parallel_agents: Option<usize>,

    /// エージェントごとのタイムアウト（秒）
    #[serde(default)]
    pub agent_timeout_sec: Option<u64>,

    /// エラー時に続行するか
    #[serde(default)]
    pub continue_on_failure: Option<bool>,

    /// 自動的に子エージェントをスポーンするか
    #[serde(default)]
    pub auto_spawn: Option<bool>,

    /// 子エージェント用のモデル
    #[serde(default)]
    pub child_agent_model: Option<String>,

    /// スペックファイルのパス（docs_dir内の特定ファイル）
    #[serde(default)]
    pub spec_file: Option<PathBuf>,
}

/// 解決済みのtacodex設定
#[derive(Debug, Clone)]
pub struct ResolvedTacodexConfig {
    /// 設定ファイルのパス（見つかった場合）
    pub config_path: Option<PathBuf>,
    /// プロジェクトルート
    pub project_root: PathBuf,
    /// ドキュメントフォルダの絶対パス
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
    /// スペックファイルの絶対パス
    pub spec_file: Option<PathBuf>,
}

impl Default for ResolvedTacodexConfig {
    fn default() -> Self {
        Self {
            config_path: None,
            project_root: PathBuf::from("."),
            docs_dir: None,
            enabled: false,
            max_parallel_agents: 4,
            agent_timeout_sec: 300,
            continue_on_failure: false,
            auto_spawn: false,
            child_agent_model: None,
            spec_file: None,
        }
    }
}

/// tacodex.tomlを探すためのマーカーファイル
const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    ".tacodex",
];

const CONFIG_FILENAME: &str = "tacodex.toml";

impl TacodexConfig {
    /// cwdから上位ディレクトリを辿ってtacodex.tomlを探す
    pub fn find_and_load(cwd: &Path) -> Result<ResolvedTacodexConfig, TacodexConfigError> {
        let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());

        // プロジェクトルートを探す
        let project_root = find_project_root(&cwd).unwrap_or_else(|| cwd.clone());

        // tacodex.tomlを探す
        let config_path = find_config_file(&cwd);

        let config = if let Some(ref path) = config_path {
            let content = std::fs::read_to_string(path)
                .map_err(|e| TacodexConfigError::ReadError(path.clone(), e))?;
            toml::from_str::<TacodexConfig>(&content)
                .map_err(|e| TacodexConfigError::ParseError(path.clone(), e))?
        } else {
            TacodexConfig::default()
        };

        Ok(config.resolve(&project_root, config_path))
    }

    /// 設定を解決する
    fn resolve(self, project_root: &Path, config_path: Option<PathBuf>) -> ResolvedTacodexConfig {
        let docs_dir = self.docs_dir.map(|d| {
            if d.is_absolute() {
                d
            } else {
                project_root.join(&d)
            }
        });

        let spec_file = self.spec_file.map(|s| {
            if s.is_absolute() {
                s
            } else if let Some(ref docs) = docs_dir {
                docs.join(&s)
            } else {
                project_root.join(&s)
            }
        });

        ResolvedTacodexConfig {
            config_path,
            project_root: project_root.to_path_buf(),
            docs_dir,
            enabled: self.enabled.unwrap_or(false),
            max_parallel_agents: self.max_parallel_agents.unwrap_or(4),
            agent_timeout_sec: self.agent_timeout_sec.unwrap_or(300),
            continue_on_failure: self.continue_on_failure.unwrap_or(false),
            auto_spawn: self.auto_spawn.unwrap_or(false),
            child_agent_model: self.child_agent_model,
            spec_file,
        }
    }
}

impl ResolvedTacodexConfig {
    /// docs_dirが有効かチェック
    pub fn is_docs_available(&self) -> bool {
        self.docs_dir
            .as_ref()
            .map(|d| d.exists() && d.is_dir())
            .unwrap_or(false)
    }

    /// スペックファイルが存在するかチェック
    pub fn has_spec_file(&self) -> bool {
        self.spec_file
            .as_ref()
            .map(|s| s.exists() && s.is_file())
            .unwrap_or(false)
    }

    /// オーケストレーションを実行すべきかチェック
    pub fn should_orchestrate(&self) -> bool {
        self.enabled && (self.is_docs_available() || self.has_spec_file())
    }
}

/// プロジェクトルートを探す
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        for marker in PROJECT_ROOT_MARKERS {
            if current.join(marker).exists() {
                return Some(current);
            }
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// tacodex.tomlを探す（cwdから上位へ）
fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let config_path = current.join(CONFIG_FILENAME);
        if config_path.exists() && config_path.is_file() {
            return Some(config_path);
        }

        // .tacodexディレクトリ内も確認
        let dotdir_config = current.join(".tacodex").join("config.toml");
        if dotdir_config.exists() && dotdir_config.is_file() {
            return Some(dotdir_config);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// tacodex設定エラー
#[derive(Debug)]
pub enum TacodexConfigError {
    ReadError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
}

impl std::fmt::Display for TacodexConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadError(path, e) => {
                write!(f, "Failed to read {}: {}", path.display(), e)
            }
            Self::ParseError(path, e) => {
                write!(f, "Failed to parse {}: {}", path.display(), e)
            }
        }
    }
}

impl std::error::Error for TacodexConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = TacodexConfig::default();
        let resolved = config.resolve(Path::new("/project"), None);

        assert!(!resolved.enabled);
        assert_eq!(resolved.max_parallel_agents, 4);
        assert_eq!(resolved.agent_timeout_sec, 300);
    }

    #[test]
    fn test_find_config() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // tacodex.tomlを作成
        let config_content = r#"
enabled = true
docs_dir = "./docs"
max_parallel_agents = 8
"#;
        std::fs::write(project_root.join("tacodex.toml"), config_content).unwrap();

        // docsディレクトリを作成
        std::fs::create_dir(project_root.join("docs")).unwrap();

        let resolved = TacodexConfig::find_and_load(project_root).unwrap();

        assert!(resolved.enabled);
        assert_eq!(resolved.max_parallel_agents, 8);
        assert!(resolved.is_docs_available());
    }

    #[test]
    fn test_relative_docs_dir() {
        let config = TacodexConfig {
            docs_dir: Some(PathBuf::from("./specifications")),
            enabled: Some(true),
            ..Default::default()
        };

        let resolved = config.resolve(Path::new("/project"), None);

        assert_eq!(
            resolved.docs_dir,
            Some(PathBuf::from("/project/specifications"))
        );
    }
}
