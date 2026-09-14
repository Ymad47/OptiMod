//! Host resource snapshot types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostSnapshot {
    pub operating_system: String,
    pub architecture: String,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub accelerators: Vec<AcceleratorSnapshot>,
    pub storage: StorageSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub model: String,
    pub physical_cores: Option<usize>,
    pub logical_cpus: usize,
    pub features: Vec<String>,
    pub numa_nodes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorSnapshot {
    pub backend: String,
    pub name: String,
    pub memory_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub filesystem: Option<String>,
    pub available_bytes: u64,
    pub rotational: Option<bool>,
}
