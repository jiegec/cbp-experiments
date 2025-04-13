use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SimPoint phase: a phase is a cluster
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimPointPhase {
    /// the number of slices in the phase
    pub weight: u64,
    /// the starting instruction of the representative slice
    pub start_instruction: u64,
    /// the ending instruction of the representative slice
    pub end_instruction: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimPointResult {
    /// Path to trace file
    pub trace_path: PathBuf,
    /// Path to executable file
    pub exe_path: PathBuf,
    /// SimPoint slice size in instructions
    pub size: u64,
    /// SimPoint phases
    pub phases: Vec<SimPointPhase>,
}
