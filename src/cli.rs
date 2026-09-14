use std::{fmt, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::hardware;

#[derive(Debug, Parser)]
#[command(
    name = "optimod",
    version,
    about = "Resource-aware llama.cpp inference advisor",
    long_about = "Inspect constrained Linux hardware, evaluate GGUF models, and recommend measured llama.cpp configurations. Model inspection, recommendations, and benchmarking remain under development."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect CPU, memory, accelerators, and model storage.
    Inspect(InspectArgs),
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
struct InspectArgs {
    /// Path whose backing storage should be inspected.
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Emit versioned machine-readable JSON.
    #[arg(long)]
    json: bool,
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

#[derive(Debug)]
pub enum CliError {
    NotImplemented(&'static str),
    Inspection(hardware::InspectionError),
    Serialization(serde_json::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented(command) => write!(formatter, "{command} is not implemented yet"),
            Self::Inspection(source) => write!(formatter, "hardware inspection failed: {source}"),
            Self::Serialization(source) => {
                write!(formatter, "could not serialize report: {source}")
            }
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inspection(source) => Some(source),
            Self::Serialization(source) => Some(source),
            Self::NotImplemented(_) => None,
        }
    }
}

impl From<hardware::InspectionError> for CliError {
    fn from(source: hardware::InspectionError) -> Self {
        Self::Inspection(source)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization(source)
    }
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect(arguments) => {
            let report = hardware::inspect(&arguments.path)?;
            if arguments.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", hardware::render_human(&report));
            }
            Ok(())
        }
        Command::Model(ModelArgs {
            command: ModelCommand::Inspect { .. },
        }) => Err(CliError::NotImplemented("model inspect")),
        Command::Recommend(_) => Err(CliError::NotImplemented("recommend")),
        Command::Benchmark(_) => Err(CliError::NotImplemented("benchmark")),
        Command::Report(_) => Err(CliError::NotImplemented("report")),
    }
}
