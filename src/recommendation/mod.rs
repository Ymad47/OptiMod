//! Recommendation contract. Policy implementation is intentionally absent.

use serde::{Deserialize, Serialize};

use crate::workload::OptimizationGoal;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateConfig {
    pub quantization: String,
    pub context_tokens: u32,
    pub concurrency: u16,
    pub threads: Option<usize>,
    pub batch_size: Option<u32>,
    pub micro_batch_size: Option<u32>,
    pub gpu_layers: Option<u32>,
    pub key_cache_type: Option<String>,
    pub value_cache_type: Option<String>,
    pub memory_map: Option<bool>,
    pub lock_memory: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub profile: OptimizationGoal,
    pub candidate: CandidateConfig,
    pub confidence: Confidence,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Estimated,
    Benchmarked,
}
