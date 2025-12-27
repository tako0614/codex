//! Markdown仕様ファイルのパーサー
//!
//! SPEC.md形式のファイルをパースして、エージェント定義を抽出する。

use std::collections::HashMap;
use std::path::Path;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
use serde::{Deserialize, Serialize};

use crate::error::OrchestratorError;

/// エージェントの種類
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// コード生成エージェント
    CodeGeneration,
    /// コードレビューエージェント
    Review,
    /// テスト実行エージェント
    Test,
    /// カスタムエージェント
    Custom,
}

impl Default for AgentType {
    fn default() -> Self {
        Self::Custom
    }
}

impl AgentType {
    /// 文字列からAgentTypeを解析
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "code_generation" | "coder" | "coding" => Self::CodeGeneration,
            "review" | "reviewer" => Self::Review,
            "test" | "tester" | "testing" => Self::Test,
            _ => Self::Custom,
        }
    }

    /// デフォルトの指示を取得
    pub fn default_instructions(&self) -> &'static str {
        match self {
            Self::CodeGeneration => r#"
あなたはコード生成の専門家です。
- クリーンで保守しやすいコードを書いてください
- 適切なエラーハンドリングを実装してください
- コメントは必要最小限に
"#,
            Self::Review => r#"
あなたはコードレビューの専門家です。
- セキュリティの問題を確認してください
- パフォーマンスの問題を指摘してください
- 改善点を具体的に提案してください
"#,
            Self::Test => r#"
あなたはテストの専門家です。
- 単体テストを書いてください
- エッジケースをカバーしてください
- テストを実行して結果を報告してください
"#,
            Self::Custom => "",
        }
    }
}

/// 個別エージェントの仕様
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// エージェント名
    pub name: String,
    /// エージェントの種類
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    /// 依存するエージェント名のリスト
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// カスタム指示
    pub instructions: Option<String>,
    /// タスクの説明
    pub task: String,
    /// 追加の設定
    #[serde(default)]
    pub config: HashMap<String, String>,
}

impl AgentSpec {
    /// 実際に使用する指示を取得
    pub fn get_instructions(&self) -> String {
        if let Some(ref custom) = self.instructions {
            custom.clone()
        } else {
            self.agent_type.default_instructions().to_string()
        }
    }
}

/// プロジェクト全体の仕様
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    /// プロジェクトのゴール
    pub goal: String,
    /// エージェント定義のマップ
    pub agents: HashMap<String, AgentSpec>,
    /// グローバル設定
    #[serde(default)]
    pub config: HashMap<String, String>,
}

impl Spec {
    /// Markdownファイルから仕様をパース
    pub fn from_markdown(content: &str) -> Result<Self, OrchestratorError> {
        let mut parser = SpecParser::new();
        parser.parse(content)?;
        parser.build()
    }

    /// ファイルから仕様を読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, OrchestratorError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| OrchestratorError::IoError(e.to_string()))?;
        Self::from_markdown(&content)
    }

    /// 依存関係を考慮した実行順序を取得（トポロジカルソート）
    pub fn execution_order(&self) -> Result<Vec<Vec<String>>, OrchestratorError> {
        let mut result: Vec<Vec<String>> = Vec::new();
        let mut completed: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut remaining: HashMap<String, &AgentSpec> = self.agents.iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();

        while !remaining.is_empty() {
            let mut batch: Vec<String> = Vec::new();

            for (name, spec) in remaining.iter() {
                let deps_satisfied = spec.depends_on.iter()
                    .all(|dep| completed.contains(dep));
                if deps_satisfied {
                    batch.push(name.clone());
                }
            }

            if batch.is_empty() && !remaining.is_empty() {
                return Err(OrchestratorError::CyclicDependency);
            }

            for name in &batch {
                remaining.remove(name);
                completed.insert(name.clone());
            }

            if !batch.is_empty() {
                result.push(batch);
            }
        }

        Ok(result)
    }
}

/// Markdownパーサーの状態
#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    Start,
    InGoal,
    InAgents,
    InAgentDef(String),
    InAgentProperty(String, String), // (agent_name, property_name)
}

/// Markdownをパースする内部構造体
struct SpecParser {
    state: ParseState,
    goal: String,
    agents: HashMap<String, AgentSpec>,
    current_text: String,
    config: HashMap<String, String>,
}

impl SpecParser {
    fn new() -> Self {
        Self {
            state: ParseState::Start,
            goal: String::new(),
            agents: HashMap::new(),
            current_text: String::new(),
            config: HashMap::new(),
        }
    }

    fn parse(&mut self, content: &str) -> Result<(), OrchestratorError> {
        let parser = Parser::new(content);
        let mut current_heading_level: Option<HeadingLevel> = None;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    self.flush_text();
                    current_heading_level = Some(level);
                    self.handle_heading_start(level);
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(level) = current_heading_level.take() {
                        self.handle_heading_end(level)?;
                    }
                }
                Event::Text(text) => {
                    self.current_text.push_str(&text);
                }
                Event::Code(code) => {
                    self.current_text.push_str(&code);
                }
                Event::SoftBreak | Event::HardBreak => {
                    self.current_text.push('\n');
                }
                Event::Start(Tag::Item) => {
                    self.handle_list_item_start();
                }
                Event::End(TagEnd::Item) => {
                    self.handle_list_item_end()?;
                }
                Event::Start(Tag::Paragraph) => {}
                Event::End(TagEnd::Paragraph) => {
                    self.handle_paragraph_end()?;
                }
                _ => {}
            }
        }

        self.flush_text();
        Ok(())
    }

    fn flush_text(&mut self) {
        self.current_text.clear();
    }

    fn handle_heading_start(&mut self, _level: HeadingLevel) {
        // ヘッダーの開始時の処理
    }

    fn handle_heading_end(&mut self, level: HeadingLevel) -> Result<(), OrchestratorError> {
        let text = self.current_text.trim().to_lowercase();
        self.current_text.clear();

        match level {
            HeadingLevel::H1 => {
                // トップレベルヘッダー: 無視またはプロジェクト名として扱う
            }
            HeadingLevel::H2 => {
                if text == "goal" || text.contains("ゴール") || text.contains("目標") {
                    self.state = ParseState::InGoal;
                } else if text == "agents" || text.contains("エージェント") {
                    self.state = ParseState::InAgents;
                }
            }
            HeadingLevel::H3 => {
                if matches!(self.state, ParseState::InAgents | ParseState::InAgentDef(_)) {
                    let agent_name = self.current_text.trim().to_string();
                    self.current_text.clear();

                    // 新しいエージェントを作成
                    let spec = AgentSpec {
                        name: agent_name.clone(),
                        agent_type: AgentType::Custom,
                        depends_on: Vec::new(),
                        instructions: None,
                        task: String::new(),
                        config: HashMap::new(),
                    };
                    self.agents.insert(agent_name.clone(), spec);
                    self.state = ParseState::InAgentDef(agent_name);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_list_item_start(&mut self) {
        self.current_text.clear();
    }

    fn handle_list_item_end(&mut self) -> Result<(), OrchestratorError> {
        let text = self.current_text.trim().to_string();
        self.current_text.clear();

        // Clone agent_name to avoid borrow conflict
        let agent_name = if let ParseState::InAgentDef(ref name) = self.state {
            Some(name.clone())
        } else {
            None
        };

        if let Some(name) = agent_name {
            self.parse_agent_property(&name, &text)?;
        }

        Ok(())
    }

    fn handle_paragraph_end(&mut self) -> Result<(), OrchestratorError> {
        let text = self.current_text.trim().to_string();

        match &self.state {
            ParseState::InGoal => {
                self.goal = text;
            }
            ParseState::InAgentDef(agent_name) => {
                // タスクの説明として追加
                if let Some(agent) = self.agents.get_mut(agent_name) {
                    if !text.is_empty() && !text.starts_with('-') {
                        if !agent.task.is_empty() {
                            agent.task.push('\n');
                        }
                        agent.task.push_str(&text);
                    }
                }
            }
            _ => {}
        }

        self.current_text.clear();
        Ok(())
    }

    fn parse_agent_property(&mut self, agent_name: &str, text: &str) -> Result<(), OrchestratorError> {
        let text = text.trim();

        if let Some((key, value)) = text.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            if let Some(agent) = self.agents.get_mut(agent_name) {
                match key.as_str() {
                    "type" => {
                        agent.agent_type = AgentType::from_str(value);
                    }
                    "depends_on" | "depends" => {
                        agent.depends_on = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "task" => {
                        agent.task = value.trim_start_matches('|').trim().to_string();
                    }
                    "instructions" => {
                        agent.instructions = Some(value.trim_start_matches('|').trim().to_string());
                    }
                    _ => {
                        agent.config.insert(key, value.to_string());
                    }
                }
            }
        }

        Ok(())
    }

    fn build(self) -> Result<Spec, OrchestratorError> {
        if self.goal.is_empty() {
            return Err(OrchestratorError::MissingGoal);
        }

        if self.agents.is_empty() {
            return Err(OrchestratorError::NoAgentsDefined);
        }

        Ok(Spec {
            goal: self.goal,
            agents: self.agents,
            config: self.config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_spec() {
        let markdown = r#"
# Project

## Goal

ユーザー認証機能を実装する

## Agents

### coder
- type: code_generation
- task: ログイン機能を実装

### reviewer
- type: review
- depends_on: coder
- task: コードをレビュー
"#;

        let spec = Spec::from_markdown(markdown).unwrap();
        assert_eq!(spec.goal, "ユーザー認証機能を実装する");
        assert_eq!(spec.agents.len(), 2);
        assert!(spec.agents.contains_key("coder"));
        assert!(spec.agents.contains_key("reviewer"));
    }

    #[test]
    fn test_execution_order() {
        let markdown = r#"
## Goal
Test

## Agents

### a
- type: custom
- task: Task A

### b
- type: custom
- depends_on: a
- task: Task B

### c
- type: custom
- depends_on: a
- task: Task C
"#;

        let spec = Spec::from_markdown(markdown).unwrap();
        let order = spec.execution_order().unwrap();

        // First batch should contain "a"
        assert!(order[0].contains(&"a".to_string()));
        // Second batch should contain "b" and "c"
        assert_eq!(order[1].len(), 2);
    }
}
