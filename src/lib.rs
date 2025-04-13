mod trace;
mod utils;

pub use ffi::*;
pub use trace::*;
pub use utils::*;

#[cxx::bridge]
mod ffi {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy)]
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

        type Predictor;

        pub fn new_predictor(name: &str) -> UniquePtr<Predictor>;
        pub fn get_prediction(self: Pin<&mut Predictor>, pc: u64) -> bool;
        pub fn update_predictor(
            self: Pin<&mut Predictor>,
            pc: u64,
            branch_type: BranchType,
            resolve_direction: bool,
            predict_direction: bool,
            branch_target: u64,
        );
        pub fn track_other_inst(
            self: Pin<&mut Predictor>,
            pc: u64,
            branch_type: BranchType,
            branch_taken: bool,
            branch_target: u64,
        );
    }
}
