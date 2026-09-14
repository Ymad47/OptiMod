//! Stable output contract for terminal and JSON renderers.

use serde::{Deserialize, Serialize};

use crate::{
    benchmark::BenchmarkResult, hardware::HostSnapshot, llama_cpp::LlamaCppInstallation,
    model::ModelProfile, recommendation::Recommendation, workload::Workload,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub schema_version: u16,
    pub host: HostSnapshot,
    pub model: ModelProfile,
    pub workload: Workload,
    pub llama_cpp: Option<LlamaCppInstallation>,
    pub recommendations: Vec<Recommendation>,
    pub benchmarks: Vec<BenchmarkResult>,
}
