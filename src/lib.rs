// follow definitions in common.h

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Branch {
    pub inst_addr: u64,
    pub targ_addr: u64,
    pub inst_length: u32,
    pub branch_type: BranchType,
}

pub struct Entry(pub u16);

impl Entry {
    pub fn get_br_index(&self) -> usize {
        (self.0 & 0x7FFF).into()
    }

    pub fn get_taken(&self) -> bool {
        (self.0 & 0x8000) != 0
    }
}

pub type BranchType = ffi::BranchType;

#[cxx::bridge]
pub mod ffi {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy)]
    pub enum BranchType {
        DirectJump,
        IndirectJump,
        DirectCall,
        IndirectCall,
        Return,
        ConditionalDirectJump,
        Invalid,
    }

    unsafe extern "C++" {
        include!("cbp-experiments/predictors/ffi/interface.h");

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
