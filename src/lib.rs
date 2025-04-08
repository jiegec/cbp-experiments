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
    inst_addr: u64,
    targ_addr: u64,
    inst_length: u32,
    branch_type: BranchType,
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
