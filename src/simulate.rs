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

    /// overall statistics
    /// number of conditional branch mispredictions
    pub total_mispred_count: u64,
    /// number of conditional branches executions
    pub total_cond_execution_count: u64,
    /// conditional branch mispredictions per kilo instructions
    pub cmpki: f64,
    /// prediction accuracy of conditional branches (%)
    pub cond_branch_prediction_accuracy: f64,
}
