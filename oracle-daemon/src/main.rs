use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

// =============================================================================
// Claude API Config
// =============================================================================

#[derive(Debug, Clone)]
struct ApiConfig {
    base_url: String,
    model: String,
    api_key: String,
}

impl ApiConfig {
    fn from_env() -> Result<Self> {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());

        let api_key = std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .context("ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY not set")?;

        Ok(Self {
            base_url,
            model,
            api_key,
        })
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

// =============================================================================
// Agent Tasks
// =============================================================================

const AGENT_TASKS: &[(&str, &str)] = &[
    ("consensus", "Review the consensus module. Analyze block validation, difficulty adjustment, merkle tree verification, timestamp validation, error handling. Focus on: crates/consensus/src/ Report issues, bugs, improvements."),
    ("network", "Review the network module. Analyze P2P protocol, peer management, message handling, connection lifecycle, security. Focus on: crates/network/src/ Report issues, bugs, improvements."),
    ("storage", "Review the storage module. Analyze RocksDB integration, data persistence, transactions, column families, error recovery. Focus on: crates/storage/src/ Report issues, bugs, improvements."),
    ("node", "Review the node module. Analyze RPC, worker coordination, chain management, API endpoints. Focus on: crates/node/src/ Report issues, bugs, improvements."),
    ("error_handling", "Review error handling. Analyze error types, propagation, Result usage, panic/unwrap usage, recovery strategies. Grep for: panic!, unwrap(), expect() Report unsafe patterns."),
    ("tests", "Review test coverage. Analyze unit tests, integration tests, completeness, edge cases, organization. Grep for: #[test], #[cfg(test)] Report test gaps."),
    ("security", "Review security. Analyze input validation, DoS protection, ban systems, rate limiting, crypto operations. Focus on: crates/network/src/security*, ban_manager*, dos_protection* Report vulnerabilities."),
    ("architecture", "Review architecture. Analyze module separation, dependencies, abstraction layers, organization, design patterns. Review Cargo.toml files. Report architectural issues."),
    ("code_quality", "Review code quality. Analyze Rust best practices, duplication, naming, documentation, clippy warnings. Look for: TODO, FIXME, HACK, XXX Report quality issues."),
    ("performance", "Review performance. Analyze bottlenecks, async usage, memory usage, parallelization, caching. Report performance issues and optimizations."),
];

// =============================================================================
// Agent Executor
// =============================================================================

struct AgentExecutor {
    client: Client,
    config: ApiConfig,
    project_path: PathBuf,
    reports_dir: PathBuf,
}

impl AgentExecutor {
    fn new(config: ApiConfig, project_path: PathBuf) -> Self {
        let reports_dir = project_path.join("oracle-reports");
        std::fs::create_dir_all(&reports_dir).ok();

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap(),
            config,
            project_path,
            reports_dir,
        }
    }

    async fn run_agent(&self, name: &str, task: &str) -> Result<String> {
        info!("Starting agent: {}", name);

        let prompt = format!(
            "You are a code reviewer analyzing the BitQuan blockchain project at: {}\n\n\
            Task: {}\n\n\
            Provide a concise report with:\n\
            1. Issues found (severity: Critical/High/Medium/Low)\n\
            2. Specific file:line references\n\
            3. Actionable recommendations\n\n\
            Be thorough but concise.",
            self.project_path.display(),
            task
        );

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: 8192,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self
            .client
            .post(self.config.messages_url())
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("API request failed")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("API error: {}", error_text);
        }

        let claude_response: ClaudeResponse = response.json().await?;
        let text = claude_response
            .content
            .iter()
            .filter_map(|b| b.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        info!(
            "Agent {} completed: {} input + {} output tokens",
            name,
            claude_response.usage.input_tokens,
            claude_response.usage.output_tokens
        );

        Ok(text)
    }

    async fn run_all_agents(&self) -> Result<Report> {
        let start = std::time::Instant::now();
        let timestamp: DateTime<Local> = Local::now();

        info!(
            "Starting hourly analysis with {} agents",
            AGENT_TASKS.len()
        );

        // Run all agents in parallel
        let mut tasks = Vec::new();
        for (name, task) in AGENT_TASKS {
            let name = name.to_string();
            let task = task.to_string();
            let executor = self.clone_ref();
            tasks.push(tokio::spawn(async move {
                executor.run_agent(&name, &task).await
            }));
        }

        // Collect results
        let mut agent_reports = Vec::new();
        let mut errors = Vec::new();

        for (idx, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(report)) => {
                    agent_reports.push((AGENT_TASKS[idx].0.to_string(), report));
                }
                Ok(Err(e)) => {
                    error!("Agent {} failed: {}", AGENT_TASKS[idx].0, e);
                    errors.push((AGENT_TASKS[idx].0.to_string(), e.to_string()));
                }
                Err(e) => {
                    error!("Agent {} panicked: {}", AGENT_TASKS[idx].0, e);
                    errors.push((
                        AGENT_TASKS[idx].0.to_string(),
                        "Panicked".to_string(),
                    ));
                }
            }
        }

        let duration = start.elapsed();

        let summary = self.generate_summary(&agent_reports);

        Ok(Report {
            timestamp: timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            duration_secs: duration.as_secs_f64(),
            agent_reports,
            errors,
            summary,
        })
    }

    fn generate_summary(&self, reports: &[(String, String)]) -> ReportSummary {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for (_, report) in reports {
            critical += report.matches("Critical").count();
            high += report.matches("High").count();
            medium += report.matches("Medium").count();
            low += report.matches("Low").count();
        }

        ReportSummary {
            total_issues: critical + high + medium + low,
            critical,
            high,
            medium,
            low,
        }
    }

    async fn save_report(&self, report: &Report) -> Result<()> {
        let filename =
            format!("report-{}.json", report.timestamp.replace(' ', "-").replace(':', "-"));
        let path = self.reports_dir.join(&filename);

        let json = serde_json::to_string_pretty(report)?;
        tokio::fs::write(&path, json).await?;

        // Also save markdown version
        let md_path = self
            .reports_dir
            .join(filename.replace(".json", ".md"));
        let markdown = self.report_to_markdown(report);
        tokio::fs::write(&md_path, markdown).await?;

        info!("Report saved to: {}", path.display());
        Ok(())
    }

    fn report_to_markdown(&self, report: &Report) -> String {
        let mut md = String::from("# BitQuan Hourly Report\n\n");
        md.push_str(&format!("**Generated:** {}\n", report.timestamp));
        md.push_str(&format!("**Duration:** {:.2}s\n", report.duration_secs));
        md.push_str("\n## Summary\n\n");
        md.push_str(&format!("- **Total Issues:** {}\n", report.summary.total_issues));
        md.push_str(&format!("- **🔴 Critical:** {}\n", report.summary.critical));
        md.push_str(&format!("- **🟠 High:** {}\n", report.summary.high));
        md.push_str(&format!("- **🟡 Medium:** {}\n", report.summary.medium));
        md.push_str(&format!("- **🟢 Low:** {}\n\n", report.summary.low));

        if !report.errors.is_empty() {
            md.push_str("## ⚠️ Agent Errors\n\n");
            for (agent, error) in &report.errors {
                md.push_str(&format!("- **{}**: {}\n", agent, error));
            }
            md.push_str("\n");
        }

        md.push_str("## Agent Reports\n\n");
        for (agent, content) in &report.agent_reports {
            md.push_str(&format!("### {}\n\n{}\n\n", agent, content));
        }

        md
    }

    fn clone_ref(&self) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap(),
            config: self.config.clone(),
            project_path: self.project_path.clone(),
            reports_dir: self.reports_dir.clone(),
        }
    }
}

// =============================================================================
// Report Types
// =============================================================================

#[derive(Debug, Serialize)]
struct Report {
    timestamp: String,
    duration_secs: f64,
    agent_reports: Vec<(String, String)>,
    errors: Vec<(String, String)>,
    summary: ReportSummary,
}

#[derive(Debug, Serialize)]
struct ReportSummary {
    total_issues: usize,
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
}

// =============================================================================
// Daemon
// =============================================================================

use clap::Parser;

#[derive(Parser, Clone)]
#[command(name = "oracle-daemon")]
#[command(about = "BitQuan project monitoring daemon", long_about = None)]
struct Cli {
    /// Project path to analyze
    #[arg(short, long, default_value = ".")]
    project_path: PathBuf,

    /// Anthropic API key (or set ANTHROPIC_API_KEY env var)
    #[arg(short, long)]
    api_key: Option<String>,

    /// Run once and exit
    #[arg(short, long)]
    once: bool,

    /// Report interval in minutes (default: 60)
    #[arg(short, long, default_value = "60")]
    interval_minutes: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Get API config from env (or override with cli arg)
    let config = if let Some(key) = cli.api_key {
        // Override API key from CLI
        let mut cfg = ApiConfig::from_env()?;
        cfg.api_key = key;
        cfg
    } else {
        ApiConfig::from_env()?
    };

    let executor = AgentExecutor::new(config, cli.project_path.clone());

    if cli.once {
        info!("Running single analysis...");
        let report = executor.run_all_agents().await?;
        executor.save_report(&report).await?;
        println!("Report generated successfully!");
        return Ok(());
    }

    info!(
        "Starting hourly daemon (interval: {} min)",
        cli.interval_minutes
    );
    let mut timer = interval(Duration::from_secs(cli.interval_minutes * 60));
    timer.tick().await; // Skip first immediate tick

    loop {
        timer.tick().await;
        info!("Starting scheduled hourly analysis...");

        match executor.run_all_agents().await {
            Ok(report) => {
                if let Err(e) = executor.save_report(&report).await {
                    error!("Failed to save report: {}", e);
                } else {
                    info!(
                        "Hourly report completed: {} issues found",
                        report.summary.total_issues
                    );
                }
            }
            Err(e) => {
                error!("Analysis failed: {}", e);
            }
        }
    }
}
