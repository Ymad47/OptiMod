//! Workload constraints used to rank configurations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workload {
    pub context_tokens: u32,
    pub maximum_output_tokens: u32,
    pub concurrency: u16,
    pub goal: OptimizationGoal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationGoal {
    MemorySaver,
    Balanced,
    QualityFirst,
    FastestMeasured,
}
