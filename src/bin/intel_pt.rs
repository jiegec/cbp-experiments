//! Parse Intel PT trace in perf.data

use capstone::{
    arch::{
        ArchOperand,
        x86::{X86Operand, X86OperandType},
    },
    prelude::*,
};
use cbp_experiments::{BranchType, get_tqdm_style};
use clap::Parser;
use memmap::MmapOptions;
use object::{Object, ObjectSection, SectionKind};
use std::{collections::BTreeMap, fs::File, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace_path: PathBuf,

    /// Path to executable file
    exe_path: PathBuf,
}

// for IP Compression
fn compute_ip_bytes(byte: u8) -> usize {
    let ip_bytes = byte >> 5;
    match ip_bytes {
        0x0 => 0,
        0x1 => 2,
        0x2 => 4,
        0x3 | 0x4 => 6,
        0x6 => 8,
        _ => unreachable!("Reserved IPBytes: {}", ip_bytes),
    }
}

fn compute_ip(data: &[u8], last_ip: u64) -> Option<u64> {
    let ip_bytes = compute_ip_bytes(data[0]);
    let mut target_ip = 0;
    for i in 0..ip_bytes {
        target_ip |= (data[i + 1] as u64) << (i * 8);
    }
    let mut result = 0;

    // combine
    match data[0] >> 5 {
        0x0 => None,
        0x1 => Some((target_ip & 0x000000000000ffff) | (last_ip & 0xffffffffffff0000)),
        0x2 => Some((target_ip & 0x00000000ffffffff) | (last_ip & 0xffffffff00000000)),
        0x3 => Some(
            (target_ip & 0x0000ffffffffffff)
                | (if (target_ip & 0x0000800000000000) != 0 {
                    0xffff000000000000
                } else {
                    0x0000000000000000
                }),
        ),
        0x4 => Some((target_ip & 0x0000ffffffffffff) | (last_ip & 0xffff000000000000)),
        0x6 => Some(target_ip),
        _ => unreachable!("Reserved IPBytes: {}", ip_bytes),
    }
}

/// Short/long TNT packet, bits are stored from newest to oldest
/// encoding:
/// 1. for 6-branch short TNT (0b1xxxxxx0), old_bit = 6, new_bit = 1
/// 2. for 4-branch short TNT (0b001xxxx0), old_bit = 4, new_bit = 1
/// 3. for 47-branch long TNT, old_bit = 62, new_bit = 16
struct TNTPacket {
    bits: [u8; 6],
    /// location of the oldest bit
    old_bit: u8,
    /// location of the newest bit
    new_bit: u8,
}

/// TIP packet, marks target address
struct TIPPacket {
    target_ip: u64,
}

enum Packet {
    TNT(TNTPacket),
    TIP(TIPPacket),
}

fn parse_intel_pt_packets(data: &[u8]) -> anyhow::Result<Vec<Packet>> {
    let mut offset = 0;
    let mut last_ip = 0;
    let mut result = vec![];
    while offset < data.len() {
        match data[offset] {
            0x00 => {
                // PAD packet
                offset += 1;
            }
            0x02 if data[offset + 1] == 0x03 => {
                // CBR packet
                offset += 4;
            }
            0x02 if data[offset + 1] == 0x23 => {
                // PSBEND packet
                offset += 2;
            }
            0x02 if data[offset + 1] == 0x73 => {
                // TMA packet
                offset += 7;
            }
            0x02 if data[offset + 1] == 0x82 => {
                // PSB packet
                last_ip = 0;
                offset += 2;
            }
            0x02 if data[offset + 1] == 0xa3 => {
                // Long TNT packet
                // find the highest 1 bit
                let mut old_bit = 0;
                for i in (2..=7).rev() {
                    if data[offset + i] != 0 {
                        let leading_zeros = data[offset + i].leading_zeros();
                        old_bit = (i + 1) as u32 * 8 - leading_zeros - 2;
                        break;
                    }
                }
                assert!(old_bit != 0);
                result.push(Packet::TNT(TNTPacket {
                    bits: [
                        data[offset + 2],
                        data[offset + 3],
                        data[offset + 4],
                        data[offset + 5],
                        data[offset + 6],
                        data[offset + 7],
                    ],
                    old_bit: old_bit as u8,
                    new_bit: 16,
                }));
                offset += 8;
            }
            0x02 if data[offset + 1] == 0xc8 => {
                // VMCS packet
                offset += 7;
            }
            0x19 => {
                // TSC packet
                offset += 8;
            }
            0x59 => {
                // MTC packet
                offset += 2;
            }
            0x99 => {
                // MODE.Exec packet
                offset += 2;
            }
            byte @ _ if byte & 0x01 == 0x00 && byte != 0x02 => {
                // Short TNT packet
                let leading_zeros = byte.leading_zeros();
                result.push(Packet::TNT(TNTPacket {
                    bits: [byte, 0, 0, 0, 0, 0],
                    old_bit: 6 - leading_zeros as u8,
                    new_bit: 1,
                }));
                offset += 1;
            }
            byte @ _
                if byte & 0x1f == 0x01
                    || byte & 0x1f == 0x0d
                    || byte & 0x1f == 0x11
                    || byte & 0x1f == 0x1d =>
            {
                // TIP.PGD(0x01)/TIP(0x0d)/TIP.PGE(0x11)/FUP packet
                if let Some(ip) = compute_ip(&data[offset..], last_ip) {
                    // TIP
                    if byte & 0x1f == 0x0d {
                        result.push(Packet::TIP(TIPPacket { target_ip: ip }));
                    }
                    last_ip = ip;
                }
                offset += 1 + compute_ip_bytes(byte);
            }
            byte @ _ => unimplemented!(
                "Unhandled packet byte: 0x{:x} at offset 0x{:x} with context {:x?}",
                byte,
                offset,
                &data[offset..std::cmp::min(offset + 16, data.len() - 1)]
            ),
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, Copy)]
pub struct BranchInfo {
    branch_type: BranchType,
    /// Target address
    targ_addr: Option<u64>,
    /// The first branch that appears after the target address
    targ_addr_branch_index: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let file = File::open(&args.trace_path)?;
    let content = unsafe { MmapOptions::new().map(&file)? };

    // parse perf.data
    println!("Parsing perf.data format");
    let magic = std::str::from_utf8(&content[..8])?;
    assert_eq!(magic, "PERFILE2");
    let mut tmp_u64 = [0u8; 8];
    let mut tmp_u32 = [0u8; 4];
    let mut tmp_u16 = [0u8; 2];
    // find data section offset
    tmp_u64.copy_from_slice(&content[40..48]);
    let data_section_offset = u64::from_le_bytes(tmp_u64) as usize;
    tmp_u64.copy_from_slice(&content[48..56]);
    let data_section_size = u64::from_le_bytes(tmp_u64) as usize;
    println!(
        "Found data section at 0x{:x}, size {}",
        data_section_offset, data_section_size
    );

    let pbar = indicatif::ProgressBar::new(data_section_size as u64);
    pbar.set_style(get_tqdm_style());
    let mut offset = data_section_offset;
    let mut packets = vec![];
    while offset < data_section_offset + data_section_size {
        tmp_u32.copy_from_slice(&content[offset..offset + 4]);
        let event_type = u32::from_le_bytes(tmp_u32);
        tmp_u16.copy_from_slice(&content[offset + 6..offset + 8]);
        let event_size = u16::from_le_bytes(tmp_u16);
        assert!(event_size > 0);
        // println!(
        //     "Got event at 0x{:x}: type {}, size {}",
        //     offset, event_type, event_size
        // );

        if event_type == 71 {
            // PERF_RECORD_AUXTRACE
            tmp_u64.copy_from_slice(&content[offset + 8..offset + 16]);
            let data_size = u64::from_le_bytes(tmp_u64) as usize;
            let data =
                &content[offset + event_size as usize..offset + event_size as usize + data_size];
            // println!(
            //     "Found Intel PT data at 0x{:x} with size {}",
            //     offset + event_size as usize,
            //     data_size
            // );
            packets.extend(parse_intel_pt_packets(data)?);
            offset += data_size;
        }
        offset += event_size as usize;

        pbar.set_position((offset - data_section_offset) as u64);
    }
    println!("Got {} packets", packets.len());

    // parse elf, find all branches and put them in an array

    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let binary_data = std::fs::read(args.exe_path)?;
    let file = object::File::parse(&*binary_data)?;
    let jump = Some("jump".to_string());
    let branch_relative = Some("branch_relative".to_string());
    let call = Some("call".to_string());
    let ret = Some("ret".to_string());
    let mut branches: Vec<BranchInfo> = vec![];
    // mapping from branch address to its index
    let mut mapping: BTreeMap<u64, usize> = BTreeMap::new();

    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                let detail: InsnDetail = cs.insn_detail(insn).expect("Failed to get insn detail");
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
                                }) => imm as u64,
                                _ => unimplemented!("Unhandled operand {:?}", ops[0]),
                            })
                        }
                        _ => None,
                    };

                    mapping.insert(insn.address(), branches.len());
                    branches.push(BranchInfo {
                        branch_type,
                        targ_addr,
                        targ_addr_branch_index: None,
                    });
                }
            }
        }
    }
    Ok(())
}
