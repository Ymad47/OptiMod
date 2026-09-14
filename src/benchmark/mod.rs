//! Repeatable benchmark result types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{metrics::InferenceMetrics, recommendation::CandidateConfig};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub model: PathBuf,
    pub candidate: CandidateConfig,
    pub run_count: u16,
    pub metrics: InferenceMetrics,
    pub notes: Vec<String>,
}
