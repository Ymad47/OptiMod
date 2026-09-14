//! Installed llama.cpp capability snapshot.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlamaCppInstallation {
    pub executable: PathBuf,
    pub version: Option<String>,
    pub build: Option<String>,
    pub supported_options: Vec<String>,
    pub available_backends: Vec<String>,
}
