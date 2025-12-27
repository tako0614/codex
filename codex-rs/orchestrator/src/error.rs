//! オーケストレーターのエラー型

use thiserror::Error;

/// オーケストレーターのエラー
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Missing goal in specification")]
    MissingGoal,

    #[error("No agents defined in specification")]
    NoAgentsDefined,

    #[error("Cyclic dependency detected in agent dependencies")]
    CyclicDependency,

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent execution failed: {agent_name} - {message}")]
    AgentExecutionFailed {
        agent_name: String,
        message: String,
    },

    #[error("Dependency failed: {agent_name} depends on {dependency}")]
    DependencyFailed {
        agent_name: String,
        dependency: String,
    },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout: agent {agent_name} exceeded time limit")]
    Timeout {
        agent_name: String,
    },

    #[error("Cancelled by user")]
    Cancelled,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for OrchestratorError {
    fn from(err: std::io::Error) -> Self {
        OrchestratorError::IoError(err.to_string())
    }
}
