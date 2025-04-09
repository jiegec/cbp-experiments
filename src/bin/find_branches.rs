/// Find and classify branches in ELF
use capstone::prelude::*;
use cbp_experiments::BranchType;
use clap::Parser;
use object::{Object, ObjectSection, SectionKind};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to ELF file
    elf: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let binary_data = std::fs::read(args.elf)?;
    let file = object::File::parse(&*binary_data)?;
    let jump = Some("jump".to_string());
    let branch_relative = Some("branch_relative".to_string());
    let call = Some("call".to_string());
    let ret = Some("ret".to_string());

    let mut branch_type_counts = [0usize; BranchType::Invalid.repr as usize];

    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                let detail: InsnDetail = cs.insn_detail(&insn).expect("Failed to get insn detail");
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
                    println!("Found {:?} branch: {}", branch_type, insn);
                    branch_type_counts[branch_type.repr as usize] += 1;
                }
            }
        }
    }

    println!("Branch counts:");
    println!(
        "- direct jump: {}",
        branch_type_counts[BranchType::DirectJump.repr as usize]
    );
    println!(
        "- indirect jump: {}",
        branch_type_counts[BranchType::IndirectJump.repr as usize]
    );
    println!(
        "- direct call: {}",
        branch_type_counts[BranchType::DirectCall.repr as usize]
    );
    println!(
        "- indirect call: {}",
        branch_type_counts[BranchType::IndirectCall.repr as usize]
    );
    println!(
        "- return: {}",
        branch_type_counts[BranchType::Return.repr as usize]
    );
    println!(
        "- conditional direct jump: {}",
        branch_type_counts[BranchType::ConditionalDirectJump.repr as usize]
    );
    Ok(())
}
