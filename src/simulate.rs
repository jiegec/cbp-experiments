use crate::{Branch, ParsedImage};
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
    /// combined simulation result might not have corresponding trace
    pub trace_path: Option<PathBuf>,
    pub images: Vec<ParsedImage>,
    pub predictor: String,

    /// skip/warmup/simulate instruction count
    pub skip: u64,
    pub warmup: u64,
    pub simulate: u64,

    /// overall statistics
    /// number of conditional branch mispredictions, pmu branch-misses
    pub total_mispred_count: u64,
    /// number of branches runtime executions, pmu branches
    pub total_br_execution_count: u64,
    /// number of conditional branches runtime executions, pmu br_inst_retired.cond
    pub total_cond_execution_count: u64,
    /// conditional branch mispredictions per kilo instructions
    pub cmpki: f64,
    /// prediction accuracy of conditional branches (%)
    pub cond_branch_prediction_accuracy: Option<f64>,

    /// per-branch statistics
    pub branch_info: Vec<SimulateResultBranchInfo>,
}
