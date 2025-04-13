use crate::Branch;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct SimulateResultBranchInfo {
    /// branch
    pub branch: Branch,
    /// statistics
    pub execution_count: u64,
    pub taken_count: u64,
    pub mispred_count: u64,
}

#[derive(Serialize, Deserialize)]
pub struct SimulateResult {
    /// configuration
    pub trace_path: PathBuf,
    pub exe_path: PathBuf,
    pub predictor: String,
    /// skip/warmup/simulate instruction count
    pub skip: usize,
    pub warmup: usize,
    pub simulate: usize,
    /// branch statistics
    pub branch_info: Vec<SimulateResultBranchInfo>,
}
