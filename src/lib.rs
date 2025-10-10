mod path;
mod simpoint;
mod simulate;
mod tage;
mod trace;
mod utils;

pub use ffi::*;
pub use path::*;
pub use simpoint::*;
pub use simulate::*;
pub use tage::*;
pub use trace::*;
pub use utils::*;

#[cxx::bridge]
mod ffi {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
    pub enum BranchType {
        /// jmp imm
        DirectJump,
        /// jmp reg/jmp mem
        IndirectJump,
        /// call imm
        DirectCall,
        /// call reg/call mem
        IndirectCall,
        /// ret
        Return,
        /// jnz imm
        ConditionalDirectJump,
        Invalid,
    }

    unsafe extern "C++" {
        include!("cbp-experiments/predictors/wrapper/interface.h");

        type ConditionalBranchPredictor;

        pub fn new_conditional_branch_predictor(
            name: &str,
        ) -> UniquePtr<ConditionalBranchPredictor>;
        pub fn list_conditional_branch_predictors() -> UniquePtr<CxxVector<CxxString>>;
        // for conditional branch:
        // 1. call get_conditional_branch_prediction to get prediction
        // 2. call update_conditional_branch_predictor to update predictor state
        // for other branches:
        // 2. call update_conditional_branch_predictor_other_inst to update predictor state
        pub fn get_conditional_branch_prediction(
            self: Pin<&mut ConditionalBranchPredictor>,
            pc: u64,
            groundtruth: bool,
        ) -> bool;
        pub fn update_conditional_branch_predictor(
            self: Pin<&mut ConditionalBranchPredictor>,
            pc: u64,
            branch_type: BranchType,
            resolve_direction: bool,
            predict_direction: bool,
            branch_target: u64,
        );
        pub fn update_conditional_branch_predictor_other_inst(
            self: Pin<&mut ConditionalBranchPredictor>,
            pc: u64,
            branch_type: BranchType,
            branch_taken: bool,
            branch_target: u64,
        );

        type IndirectBranchPredictor;

        pub fn new_indirect_branch_predictor(name: &str) -> UniquePtr<IndirectBranchPredictor>;
        pub fn list_indirect_branch_predictors() -> UniquePtr<CxxVector<CxxString>>;
        // for indirect branch:
        // 1. call get_indirect_branch_prediction
        // 2. call update_indirect_branch_predictor to update predictor state
        // for other branches:
        // 2. call update_indirect_branch_predictor to update predictor state
        pub fn get_indirect_branch_prediction(
            self: Pin<&mut IndirectBranchPredictor>,
            pc: u64,
            branch_type: BranchType,
            groundtruth: u64,
        ) -> u64;
        pub fn update_indirect_branch_predictor(
            self: Pin<&mut IndirectBranchPredictor>,
            pc: u64,
            branch_type: BranchType,
            taken: bool,
            branch_target: u64,
        );
    }
}

pub fn is_indirect(br: BranchType) -> bool {
    br == BranchType::IndirectCall || br == BranchType::IndirectJump
}
