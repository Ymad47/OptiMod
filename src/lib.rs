//! OptiMod library boundaries.
//!
//! Runtime inspection and recommendation behavior will be added behind
//! these modules without coupling policy to operating-system probes.

pub mod benchmark;
pub mod cli;
pub mod hardware;
pub mod llama_cpp;
pub mod metrics;
pub mod model;
pub mod recommendation;
pub mod report;
pub mod workload;
