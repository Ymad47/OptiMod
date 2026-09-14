use std::{fmt, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "optimod",
    version,
    about = "Resource-aware llama.cpp inference advisor",
    long_about = "Inspect constrained hardware, evaluate GGUF models, and recommend measured llama.cpp configurations. Runtime behavior is not implemented in this scaffold release."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect CPU, memory, accelerators, and model storage.
    Inspect,
    /// Inspect model metadata.
    Model(ModelArgs),
    /// Rank candidate inference configurations.
    Recommend(RecommendArgs),
    /// Benchmark a model with llama.cpp.
    Benchmark(BenchmarkArgs),
    /// Render the latest optimization report.
    Report(ReportArgs),
}

#[derive(Debug, Args)]
struct ModelArgs {
    #[command(subcommand)]
    command: ModelCommand,
}

#[derive(Debug, Subcommand)]
enum ModelCommand {
    /// Inspect metadata from a GGUF model.
    Inspect { model: PathBuf },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Profile {
    MemorySaver,
    Balanced,
    QualityFirst,
    FastestMeasured,
}

#[derive(Debug, Args)]
struct RecommendArgs {
    /// GGUF model to evaluate.
    model: PathBuf,
    /// Optimization profile.
    #[arg(long, value_enum, default_value_t = Profile::Balanced)]
    profile: Profile,
    /// Requested context window in tokens.
    #[arg(long)]
    context_tokens: Option<u32>,
    /// Concurrent inference requests.
    #[arg(long, default_value_t = 1)]
    concurrency: u16,
    /// Explicit llama.cpp executable.
    #[arg(long)]
    llama_cpp: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct BenchmarkArgs {
    /// GGUF model to benchmark.
    model: PathBuf,
    /// Number of measured runs.
    #[arg(long, default_value_t = 3)]
    runs: u16,
    /// Explicit llama-bench executable.
    #[arg(long)]
    llama_bench: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ReportArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    NotImplemented(&'static str),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented(command) => write!(
                formatter,
                "{command} is not implemented in the scaffold release"
            ),
        }
    }
}

impl std::error::Error for CliError {}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let command = match cli.command {
        Command::Inspect => "inspect",
        Command::Model(ModelArgs {
            command: ModelCommand::Inspect { .. },
        }) => "model inspect",
        Command::Recommend(_) => "recommend",
        Command::Benchmark(_) => "benchmark",
        Command::Report(_) => "report",
    };

    Err(CliError::NotImplemented(command))
}
