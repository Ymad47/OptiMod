//! Resource and inference measurements.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceMetrics {
    pub cold_load_milliseconds: Option<u64>,
    pub warm_load_milliseconds: Option<u64>,
    pub prompt_tokens_per_second: Option<f64>,
    pub generated_tokens_per_second: Option<f64>,
    pub time_to_first_token_milliseconds: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
    pub bytes_read: Option<u64>,
}
