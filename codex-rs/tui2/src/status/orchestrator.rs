//! オーケストレーターステータスパネル
//!
//! 子エージェントの状態を表示するTUIウィジェット。

use ratatui::prelude::*;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// 子エージェントの表示用ステータス
#[derive(Debug, Clone, PartialEq)]
pub enum AgentDisplayStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

impl AgentDisplayStatus {
    /// ステータスのアイコンを取得
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Running => "◐",
            Self::Completed => "✓",
            Self::Failed(_) => "✗",
            Self::Cancelled => "⊘",
        }
    }

    /// ステータスの色を取得
    pub fn color(&self) -> Color {
        match self {
            Self::Pending => Color::Gray,
            Self::Running => Color::Yellow,
            Self::Completed => Color::Green,
            Self::Failed(_) => Color::Red,
            Self::Cancelled => Color::DarkGray,
        }
    }
}

/// 子エージェントの表示情報
#[derive(Debug, Clone)]
pub struct AgentDisplayInfo {
    pub name: String,
    pub status: AgentDisplayStatus,
    pub task: String,
    pub progress: Option<String>,
    pub elapsed_ms: u64,
}

/// オーケストレーターの表示状態
#[derive(Debug, Clone, Default)]
pub struct OrchestratorDisplayState {
    pub enabled: bool,
    pub active: bool,
    pub docs_dir: Option<String>,
    pub agents: Vec<AgentDisplayInfo>,
    pub completed_count: usize,
    pub failed_count: usize,
    pub message: String,
}

impl OrchestratorDisplayState {
    /// パネルを表示すべきか
    pub fn should_show(&self) -> bool {
        self.enabled && (self.active || !self.agents.is_empty())
    }

    /// 進捗率を取得（0.0 - 1.0）
    pub fn progress_ratio(&self) -> f64 {
        if self.agents.is_empty() {
            return 0.0;
        }
        let done = self.completed_count + self.failed_count;
        done as f64 / self.agents.len() as f64
    }
}

/// オーケストレーターパネルウィジェット
pub struct OrchestratorPanel<'a> {
    state: &'a OrchestratorDisplayState,
}

impl<'a> OrchestratorPanel<'a> {
    pub fn new(state: &'a OrchestratorDisplayState) -> Self {
        Self { state }
    }
}

impl Widget for OrchestratorPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.should_show() {
            return;
        }

        // タイトルを構築
        let title = format!(
            " Agents [{}/{}] ",
            self.state.completed_count + self.state.failed_count,
            self.state.agents.len()
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // エージェント一覧を表示
        let items: Vec<ListItem> = self
            .state
            .agents
            .iter()
            .map(|agent| {
                let icon = agent.status.icon();
                let color = agent.status.color();

                let elapsed = if agent.elapsed_ms > 0 {
                    format!(" ({}ms)", agent.elapsed_ms)
                } else {
                    String::new()
                };

                let progress_text = agent
                    .progress
                    .as_ref()
                    .map(|p| format!(" - {}", p))
                    .unwrap_or_default();

                let line = Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(&agent.name, Style::default().bold()),
                    Span::styled(elapsed, Style::default().fg(Color::DarkGray)),
                    Span::styled(progress_text, Style::default().fg(Color::Gray)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, inner, buf);
    }
}

/// オーケストレーターステータスバーウィジェット（コンパクト表示）
pub struct OrchestratorStatusBar<'a> {
    state: &'a OrchestratorDisplayState,
}

impl<'a> OrchestratorStatusBar<'a> {
    pub fn new(state: &'a OrchestratorDisplayState) -> Self {
        Self { state }
    }
}

impl Widget for OrchestratorStatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.enabled {
            return;
        }

        let running = self
            .state
            .agents
            .iter()
            .filter(|a| matches!(a.status, AgentDisplayStatus::Running))
            .count();

        let text = if self.state.active {
            format!(
                "🤖 Agents: {} running, {}/{} done",
                running,
                self.state.completed_count,
                self.state.agents.len()
            )
        } else if !self.state.agents.is_empty() {
            format!(
                "🤖 {}/{} agents completed",
                self.state.completed_count,
                self.state.agents.len()
            )
        } else {
            "🤖 Orchestrator ready".to_string()
        };

        let paragraph = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_display_status() {
        assert_eq!(AgentDisplayStatus::Completed.icon(), "✓");
        assert_eq!(AgentDisplayStatus::Running.icon(), "◐");
    }

    #[test]
    fn test_progress_ratio() {
        let state = OrchestratorDisplayState {
            agents: vec![
                AgentDisplayInfo {
                    name: "a".to_string(),
                    status: AgentDisplayStatus::Completed,
                    task: "".to_string(),
                    progress: None,
                    elapsed_ms: 0,
                },
                AgentDisplayInfo {
                    name: "b".to_string(),
                    status: AgentDisplayStatus::Running,
                    task: "".to_string(),
                    progress: None,
                    elapsed_ms: 0,
                },
            ],
            completed_count: 1,
            failed_count: 0,
            ..Default::default()
        };

        assert_eq!(state.progress_ratio(), 0.5);
    }
}
