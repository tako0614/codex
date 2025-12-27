//! `codex spec` サブコマンドの実装
//!
//! 仕様ファイルを読み込んで、複数のエージェントを並列実行する。

use std::path::PathBuf;

use clap::Parser;
use codex_common::CliConfigOverrides;
use codex_orchestrator::{
    Spec, Orchestrator, OrchestratorConfig, CodexRunner,
    AgentEvent, OrchestrationResult,
};
use owo_colors::OwoColorize;

/// Run Codex with a specification file (multi-agent mode).
#[derive(Debug, Parser)]
pub struct SpecCli {
    /// Path to the specification file (default: SPEC.md)
    #[arg(default_value = "SPEC.md")]
    pub spec_file: PathBuf,

    /// Model to use for agents
    #[arg(short, long)]
    pub model: Option<String>,

    /// Maximum number of parallel agents
    #[arg(long, default_value = "4")]
    pub parallelism: usize,

    /// Agent timeout in seconds
    #[arg(long, default_value = "300")]
    pub timeout: u64,

    /// Continue even if some agents fail
    #[arg(long)]
    pub continue_on_failure: bool,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Dry run - parse spec and show execution plan without running
    #[arg(long)]
    pub dry_run: bool,

    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,
}

/// spec コマンドを実行
pub async fn run_spec_command(cli: SpecCli) -> anyhow::Result<()> {
    // 仕様ファイルを読み込む
    println!(
        "{} {}",
        "Reading specification:".bold(),
        cli.spec_file.display()
    );

    let spec = Spec::from_file(&cli.spec_file).map_err(|e| {
        anyhow::anyhow!("Failed to parse specification file: {}", e)
    })?;

    println!("{} {}", "Goal:".bold().green(), spec.goal);
    println!("{} {}", "Agents:".bold().green(), spec.agents.len());

    // 実行順序を表示
    let execution_order = spec.execution_order().map_err(|e| {
        anyhow::anyhow!("Failed to determine execution order: {}", e)
    })?;

    println!("\n{}", "Execution Plan:".bold().cyan());
    for (batch_idx, batch) in execution_order.iter().enumerate() {
        println!(
            "  Batch {}: {}",
            batch_idx + 1,
            batch.join(", ").yellow()
        );
    }

    // dry-run モードならここで終了
    if cli.dry_run {
        println!("\n{}", "Dry run complete. No agents were executed.".bold());
        return Ok(());
    }

    println!("\n{}", "Starting orchestration...".bold().green());

    // オーケストレーター設定
    let config = OrchestratorConfig {
        working_dir: std::env::current_dir()?,
        max_parallelism: cli.parallelism,
        agent_timeout_secs: cli.timeout,
        continue_on_failure: cli.continue_on_failure,
    };

    // ランナーを作成
    let model = cli.model.unwrap_or_else(|| "gpt-4".to_string());
    let runner = CodexRunner::new(model);

    // オーケストレーターを作成
    let mut orchestrator = Orchestrator::new(spec, config, runner);

    // イベントハンドラーを設定
    let event_rx = orchestrator.take_event_receiver();

    // イベント処理タスクを起動
    let verbose = cli.verbose;
    let event_handle = tokio::spawn(async move {
        if let Some(mut rx) = event_rx {
            while let Some(event) = rx.recv().await {
                handle_event(event, verbose);
            }
        }
    });

    // オーケストレーションを実行
    let result = orchestrator.run().await.map_err(|e| {
        anyhow::anyhow!("Orchestration failed: {}", e)
    })?;

    // イベントハンドラーを待機
    drop(orchestrator);
    let _ = event_handle.await;

    // 結果を表示
    print_result(&result);

    if result.is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some agents failed"))
    }
}

/// エージェントイベントを処理
fn handle_event(event: AgentEvent, verbose: bool) {
    match event {
        AgentEvent::Started { agent_name } => {
            println!(
                "  {} {}",
                "▶".green(),
                format!("Agent '{}' started", agent_name).bold()
            );
        }
        AgentEvent::Progress { agent_name, message } => {
            if verbose {
                println!(
                    "    {} [{}] {}",
                    "•".blue(),
                    agent_name.dimmed(),
                    message
                );
            }
        }
        AgentEvent::ToolCall { agent_name, tool_name, .. } => {
            if verbose {
                println!(
                    "    {} [{}] Calling tool: {}",
                    "⚙".yellow(),
                    agent_name.dimmed(),
                    tool_name
                );
            }
        }
        AgentEvent::ToolResult { agent_name, tool_name, .. } => {
            if verbose {
                println!(
                    "    {} [{}] Tool '{}' completed",
                    "✓".green(),
                    agent_name.dimmed(),
                    tool_name
                );
            }
        }
        AgentEvent::Completed { agent_name, result } => {
            let status_icon = if matches!(result.status, codex_orchestrator::AgentStatus::Completed) {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            println!(
                "  {} {} ({}ms)",
                status_icon,
                format!("Agent '{}' completed", agent_name).bold(),
                result.execution_time_ms
            );
        }
        AgentEvent::Error { agent_name, error } => {
            println!(
                "  {} {} - {}",
                "✗".red(),
                format!("Agent '{}' error", agent_name).bold().red(),
                error
            );
        }
    }
}

/// 結果を表示
fn print_result(result: &OrchestrationResult) {
    println!("\n{}", "═".repeat(50));
    println!("{}", "Orchestration Complete".bold().cyan());
    println!("{}", "═".repeat(50));

    println!(
        "  {} {}",
        "Succeeded:".green(),
        result.succeeded.to_string().bold()
    );
    println!(
        "  {} {}",
        "Failed:".red(),
        result.failed.to_string().bold()
    );
    println!(
        "  {} {}ms",
        "Total Time:".blue(),
        result.total_time_ms.to_string().bold()
    );

    if !result.results.is_empty() {
        println!("\n{}", "Agent Results:".bold());
        for (name, agent_result) in &result.results {
            let status_str = match &agent_result.status {
                codex_orchestrator::AgentStatus::Completed => "✓ Completed".green().to_string(),
                codex_orchestrator::AgentStatus::Failed(msg) => {
                    format!("✗ Failed: {}", msg).red().to_string()
                }
                codex_orchestrator::AgentStatus::Cancelled => "⊘ Cancelled".yellow().to_string(),
                _ => format!("{:?}", agent_result.status),
            };
            println!("  {} {}", name.bold(), status_str);
        }
    }

    println!("{}", "═".repeat(50));

    if result.is_success() {
        println!("{}", "All agents completed successfully! 🎉".bold().green());
    } else {
        println!("{}", "Some agents failed. Check the output above.".bold().red());
    }
}
