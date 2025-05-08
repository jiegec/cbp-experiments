use capstone::{
    arch::{
        ArchOperand,
        x86::{X86Operand, X86OperandType},
    },
    prelude::*,
};
use object::{Architecture, Object, ObjectKind, ObjectSection, SectionKind};
use std::{collections::HashMap, path::Path};

use crate::{BranchType, Image};

pub fn get_tqdm_style() -> indicatif::ProgressStyle {
    indicatif::ProgressStyle::with_template(
            "{percent:>3}% |{wide_bar}| {pos}/{len} [{elapsed_precise}<{eta_precise}, {custom_per_sec}]",
        )
        .unwrap()
        .with_key(
            "custom_per_sec",
            Box::new(|s: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.2e} it/s", s.per_sec()).unwrap()),
        ).progress_chars("██ ")
}

/// create a mapping from instruction address to instruction index for instruction counting
pub fn create_inst_index_mapping<P: AsRef<std::path::Path>>(
    elf: P,
) -> anyhow::Result<HashMap<u64, u64>> {
    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let mut mapping: HashMap<u64, u64> = HashMap::new();
    let binary_data = std::fs::read(elf)?;
    let file = object::File::parse(&*binary_data)?;

    let mut i = 0;
    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                assert_eq!(mapping.insert(insn.address(), i), None);
                i += 1;
            }
        }
    }
    Ok(mapping)
}

/// create a mapping from instruction address to instruction index for instruction counting
pub fn create_inst_index_mapping_from_images(
    images: &[Image],
) -> anyhow::Result<HashMap<u64, u64>> {
    let mut addrs = vec![];
    for image in images {
        let mut image_filename = image.get_filename()?;
        // parse instructions in the image
        if image_filename == "[vdso]" {
            // use our dumped vdso
            image_filename = "tracers/intel-pt/vdso".to_string();
        }
        let binary_data = std::fs::read(&image_filename)?;
        let file = object::File::parse(&*binary_data)?;
        let load_base = match file.kind() {
            ObjectKind::Executable => 0,
            ObjectKind::Dynamic => image.start,
            _ => unimplemented!("Unsupported file kind"),
        };
        let cs = match file.architecture() {
            Architecture::X86_64 => Capstone::new()
                .x86()
                .mode(arch::x86::ArchMode::Mode64)
                .syntax(arch::x86::ArchSyntax::Att)
                .detail(true)
                .build()?,
            Architecture::Aarch64 => Capstone::new()
                .arm64()
                .mode(arch::arm64::ArchMode::Arm)
                .detail(true)
                .build()?,
            _ => unimplemented!("Unsupported architecture"),
        };

        for section in file.sections() {
            if section.kind() == SectionKind::Text {
                let content = section.data()?;
                let insns = cs.disasm_all(content, section.address())?;
                for insn in insns.as_ref() {
                    let addr = insn.address() + load_base;
                    addrs.push(addr);
                }
            }
        }
    }

    // assign index from low to high address
    addrs.sort();

    let mut mapping: HashMap<u64, u64> = HashMap::new();
    let mut i = 0;
    for addr in addrs {
        assert_eq!(mapping.insert(addr, i), None);
        i += 1;
    }
    println!(
        "Found {} static instructions from {} images",
        i,
        images.len()
    );
    Ok(mapping)
}

pub fn get_inst_index(mapping: &HashMap<u64, u64>, addr: u64) -> u64 {
    match mapping.get(&addr) {
        Some(index) => {
            // found
            *index
        }
        None => {
            panic!("Failed to get instruction index for pc 0x{:x}", addr);
        }
    }
}

/// Static branches parsed from ELF
#[derive(Debug, Clone, Copy)]
pub struct StaticBranch {
    pub inst_addr: u64,
    pub targ_addr: Option<u64>, // only available for direct branches
    pub inst_length: u32,
    pub branch_type: BranchType,
}

/// Find all branches by parsing ELF
pub fn find_branches<P: AsRef<Path>>(path: P, load_base: u64) -> anyhow::Result<Vec<StaticBranch>> {
    let mut branches = vec![];

    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let binary_data = std::fs::read(path)?;
    let file = object::File::parse(&*binary_data)?;
    let jump = Some("jump".to_string());
    let branch_relative = Some("branch_relative".to_string());
    let call = Some("call".to_string());
    let ret = Some("ret".to_string());

    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                let detail: InsnDetail = cs.insn_detail(insn)?;
                let groups: Vec<Option<String>> = detail
                    .groups()
                    .iter()
                    .map(|id| cs.group_name(*id))
                    .collect();
                let has_jump = groups.contains(&jump);
                let has_branch_relative = groups.contains(&branch_relative);
                let has_call = groups.contains(&call);
                let has_ret = groups.contains(&ret);
                if has_jump || has_branch_relative || has_call || has_ret {
                    // classify
                    let mnemonic = insn.mnemonic().unwrap();
                    let branch_type = match (has_jump, has_branch_relative, has_call, has_ret) {
                        // direct jump, possible conditional
                        (true, true, false, false) => match mnemonic {
                            "jmp" => BranchType::DirectJump,
                            "ja" | "jae" | "jb" | "jbe" | "jc" | "jcxz" | "jecxz" | "jrcxz"
                            | "je" | "jg" | "jge" | "jl" | "jle" | "jna" | "jnae" | "jnb"
                            | "jnbe" | "jnc" | "jne" | "jng" | "jnge" | "jnl" | "jnle" | "jno"
                            | "jnp" | "jns" | "jnz" | "jo" | "jp" | "jpe" | "jpo" | "js" | "jz" => {
                                BranchType::ConditionalDirectJump
                            }
                            "xbegin" => continue,
                            _ => unimplemented!("Unhandled mnemonic {}", mnemonic),
                        },
                        // indirect jump
                        (true, false, false, false) => {
                            assert!(["jmpq"].contains(&mnemonic));
                            BranchType::IndirectJump
                        }
                        // direct call
                        (false, true, true, false) => {
                            assert_eq!(mnemonic, "callq");
                            BranchType::DirectCall
                        }
                        // indirect call
                        (false, false, true, false) => {
                            assert_eq!(mnemonic, "callq");
                            BranchType::IndirectCall
                        }
                        // return
                        (false, false, false, true) => {
                            assert!(["retq"].contains(&mnemonic));
                            BranchType::Return
                        }
                        _ => unimplemented!("Unhandled insn {} with groups {:?}", insn, groups),
                    };

                    let ops = detail.arch_detail().operands();
                    let targ_addr = match branch_type {
                        BranchType::ConditionalDirectJump
                        | BranchType::DirectCall
                        | BranchType::DirectJump => {
                            assert_eq!(ops.len(), 1);
                            Some(match ops[0] {
                                ArchOperand::X86Operand(X86Operand {
                                    op_type: X86OperandType::Imm(imm),
                                    size: _,
                                    access: _,
                                    avx_bcast: _,
                                    avx_zero_opmask: _,
                                }) => {
                                    // add runtime load offset
                                    imm as u64 + load_base
                                }
                                _ => unimplemented!("Unhandled operand {:?}", ops[0]),
                            })
                        }
                        _ => None,
                    };

                    // add runtime load offset
                    let inst_addr = insn.address() + load_base;

                    branches.push(StaticBranch {
                        branch_type,
                        inst_addr,
                        inst_length: insn.len() as u32,
                        targ_addr,
                    });
                }
            }
        }
    }
    Ok(branches)
}
