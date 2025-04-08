// follow definitions in common.h

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
