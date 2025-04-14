//! Parse Intel PT trace in perf.data and convert to our trace format

use capstone::{
    arch::{
        ArchOperand,
        x86::{X86Operand, X86OperandType},
    },
    prelude::*,
};
use cbp_experiments::{BranchType, Image, TraceFileEncoder, get_tqdm_style};
use clap::Parser;
use indicatif::ProgressBar;
use memmap::{Mmap, MmapOptions};
use object::{Object, ObjectSection, SectionKind};
use std::{
    collections::VecDeque,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to input trace file (perf.data)
    #[arg(short, long)]
    trace_path: PathBuf,

    /// Path to executable file
    #[arg(short, long)]
    exe_path: PathBuf,

    /// Path to output trace file
    #[arg(short, long)]
    output_path: PathBuf,
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
/// 3. for 47-branch long TNT, old_bit = 46, new_bit = 0
#[derive(Clone, Copy)]
pub struct TNTPacket {
    bits: [u8; 6],
    /// location of the oldest bit
    old_bit: u8,
    /// location of the newest bit
    new_bit: u8,
}

impl std::fmt::Debug for TNTPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bit in (self.new_bit..=self.old_bit).rev() {
            let taken = ((self.bits[(bit / 8) as usize] >> (bit % 8)) & 1) != 0;
            write!(f, "{}", if taken { "T" } else { "N" })?;
        }
        Ok(())
    }
}

/// TIP packet, marks target address
#[derive(Clone, Copy, Debug)]
pub struct TIPPacket {
    target_ip: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum Packet {
    TNT(TNTPacket),
    TIP(TIPPacket),
}

fn parse_intel_pt_packets(data: &[u8]) -> Vec<Packet> {
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
                        old_bit = (i - 1) as u32 * 8 - leading_zeros - 2;
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
                    new_bit: 0,
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
            byte if byte & 0x01 == 0x00 && byte != 0x02 => {
                // Short TNT packet
                let leading_zeros = byte.leading_zeros();
                result.push(Packet::TNT(TNTPacket {
                    bits: [byte, 0, 0, 0, 0, 0],
                    old_bit: 6 - leading_zeros as u8,
                    new_bit: 1,
                }));
                offset += 1;
            }
            byte if byte & 0x1f == 0x01
                || byte & 0x1f == 0x0d
                || byte & 0x1f == 0x11
                || byte & 0x1f == 0x1d =>
            {
                // TIP.PGD(0x01)/TIP(0x0d)/TIP.PGE(0x11)/FUP(0x1d) packet
                if let Some(ip) = compute_ip(&data[offset..], last_ip) {
                    // TIP
                    if byte & 0x1f == 0x0d {
                        result.push(Packet::TIP(TIPPacket { target_ip: ip }));
                    }
                    last_ip = ip;
                }
                offset += 1 + compute_ip_bytes(byte);
            }
            byte => unimplemented!(
                "Unhandled packet byte: 0x{:x} at offset 0x{:x} with context {:x?}",
                byte,
                offset,
                &data[offset..std::cmp::min(offset + 16, data.len() - 1)]
            ),
        }
    }
    result
}

#[derive(Debug, Clone, Copy)]
pub struct BranchInfo {
    /// Branch address
    inst_addr: u64,
    /// Branch type
    branch_type: BranchType,
    /// Instruction length
    inst_length: u32,
    /// Target address
    targ_addr: Option<u64>,
    /// The first branch that appears after the target address
    targ_addr_branch_index: Option<usize>,
}

impl BranchInfo {
    /// Fallthrough address, for call stack maintenance
    pub fn fall_addr(&self) -> u64 {
        self.inst_addr + self.inst_length as u64
    }
}

pub struct IntelPTIterator<'a> {
    content: Mmap,
    pbar: ProgressBar,
    offset: usize,
    data_begin: usize,
    data_end: usize,
    packets: VecDeque<Packet>,

    // collect mmaped files on the fly
    images: &'a mut Vec<Image>,
}

impl<'a> Iterator for IntelPTIterator<'a> {
    type Item = Packet;

    fn next(&mut self) -> Option<Self::Item> {
        // scan for more packets
        while self.packets.is_empty() && self.offset < self.data_end {
            let mut tmp_u64 = [0u8; 8];
            let mut tmp_u32 = [0u8; 4];
            let mut tmp_u16 = [0u8; 2];
            tmp_u32.copy_from_slice(&self.content[self.offset..self.offset + 4]);
            let event_type = u32::from_le_bytes(tmp_u32);
            tmp_u16.copy_from_slice(&self.content[self.offset + 6..self.offset + 8]);
            let event_size = u16::from_le_bytes(tmp_u16);
            assert!(event_size > 0);
            // println!(
            //     "Got event at 0x{:x}: type {}, size {}",
            //     offset, event_type, event_size
            // );

            if event_type == 71 {
                // PERF_RECORD_AUXTRACE
                // see struct perf_record_auxtrace in linux kernel
                // 8 byte header
                // 8 byte size
                tmp_u64.copy_from_slice(&self.content[self.offset + 8..self.offset + 16]);
                let data_size = u64::from_le_bytes(tmp_u64) as usize;
                let data = &self.content[self.offset + event_size as usize
                    ..self.offset + event_size as usize + data_size];
                // println!(
                //     "Found Intel PT data at 0x{:x} with size {}",
                //     offset + event_size as usize,
                //     data_size
                // );
                self.packets.extend(parse_intel_pt_packets(data));
                self.offset += data_size;
            } else if event_type == 10 {
                // PERF_RECORD_MMAP2
                // see struct perf_record_mmap2 in linux kernel
                // 8 byte header
                // 4 byte pid
                // 4 byte tid
                // 8 byte start @ 0x10
                tmp_u64.copy_from_slice(&self.content[self.offset + 16..self.offset + 24]);
                let start = u64::from_le_bytes(tmp_u64);
                // 8 byte len @ 0x18
                tmp_u64.copy_from_slice(&self.content[self.offset + 24..self.offset + 32]);
                let len = u64::from_le_bytes(tmp_u64);
                // 8 byte offset @ 0x20
                tmp_u64.copy_from_slice(&self.content[self.offset + 32..self.offset + 40]);
                let offset = u64::from_le_bytes(tmp_u64);
                // read 256 bytes from filename field @ 0x48
                let mut filename = [0u8; 256];
                filename.copy_from_slice(&self.content[self.offset + 72..self.offset + 328]);
                self.images.push(Image {
                    start: start - offset,
                    len: len + offset,
                    filename,
                });
            }
            self.offset += event_size as usize;

            self.pbar
                .set_position((self.offset - self.data_begin) as u64);
        }

        if let Some(packet) = self.packets.pop_front() {
            return Some(packet);
        }
        None
    }
}

impl<'a> IntelPTIterator<'a> {
    pub fn from<P: AsRef<Path>>(path: P, images: &'a mut Vec<Image>) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let content = unsafe { MmapOptions::new().map(&file)? };

        // parse perf.data
        println!("Parsing perf.data format");
        let magic = std::str::from_utf8(&content[..8])?;
        assert_eq!(magic, "PERFILE2");
        let mut tmp_u64 = [0u8; 8];
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

        images.clear();

        Ok(Self {
            content,
            pbar,
            offset: data_section_offset,
            data_begin: data_section_offset,
            data_end: data_section_offset + data_section_size,
            packets: VecDeque::new(),
            images,
        })
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

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

                    // ensure monotonicity
                    let inst_addr = insn.address();
                    assert!(
                        inst_addr > branches.last().map(|branch| branch.inst_addr).unwrap_or(0)
                    );

                    branches.push(BranchInfo {
                        branch_type,
                        inst_addr,
                        inst_length: insn.len() as u32,
                        targ_addr,
                        targ_addr_branch_index: None,
                    });
                }
            }
        }
    }

    // find the first branch that appears after or equal to the target address
    let find_branch_by_pc = |branches: &Vec<BranchInfo>, pc: u64| {
        match branches.binary_search_by_key(&pc, |info| info.inst_addr) {
            Ok(index) => {
                // exact match
                index
            }
            Err(index) => {
                // the immediate next
                index
            }
        }
    };

    // pre-process target branch indices
    // so that we can locate the next branch quickly
    let targ_addr_branch_indices: Vec<Option<usize>> = branches
        .iter()
        .map(|branch| {
            if let Some(targ_addr) = branch.targ_addr {
                return Some(find_branch_by_pc(&branches, targ_addr));
            }

            None
        })
        .collect();
    for (branch, targ_addr_branch_index) in branches.iter_mut().zip(targ_addr_branch_indices) {
        branch.targ_addr_branch_index = targ_addr_branch_index;
    }

    // starting from entrypoint, iterate branches
    let entry_pc = file.entry();
    let mut branch_index = find_branch_by_pc(&branches, entry_pc);
    // maintain call stack of depth 64, storing return address & next branch index of calls
    let mut call_stack: VecDeque<(u64, usize)> = VecDeque::new();

    println!("Reconstructing control from entrypoint 0x{:x}", entry_pc);
    println!("Writing to trace file at {}", args.output_path.display());
    let output_file = File::create(&args.output_path)?;
    let mut output_trace = TraceFileEncoder::open(&output_file)?;

    // Maintain branch index in output file as optimization
    let mut output_branch_indices: Vec<Option<usize>> = vec![None; branches.len()];

    // record direct branch, eligible for caching branch index in output trace
    let record_direct = |output_trace: &mut TraceFileEncoder,
                         branch: &BranchInfo,
                         output_branch_index: &mut Option<usize>,
                         taken: bool| match output_branch_index {
        Some(branch_index) => output_trace.record_event_with_branch_index(*branch_index, taken),
        None => {
            let new_branch_index = output_trace.record_event(
                branch.inst_addr,
                branch.targ_addr.unwrap(),
                branch.inst_length,
                branch.branch_type,
                taken,
            )?;
            *output_branch_index = Some(new_branch_index);
            Ok(())
        }
    };

    // record indirect branch
    let record_indirect =
        |output_trace: &mut TraceFileEncoder, branch: &BranchInfo, targ_addr: u64| {
            output_trace.record_event(
                branch.inst_addr,
                targ_addr,
                branch.inst_length,
                branch.branch_type,
                true,
            )
        };

    let mut images = vec![];
    for packet in IntelPTIterator::from(args.trace_path, &mut images)? {
        match packet {
            Packet::TNT(tnt) => {
                for bit in (tnt.new_bit..=tnt.old_bit).rev() {
                    let taken = ((tnt.bits[(bit / 8) as usize] >> (bit % 8)) & 1) != 0;

                    // loop until we found the conditional branch
                    loop {
                        let branch = &branches[branch_index];

                        match branch.branch_type {
                            BranchType::ConditionalDirectJump => {
                                record_direct(
                                    &mut output_trace,
                                    branch,
                                    &mut output_branch_indices[branch_index],
                                    taken,
                                )?;

                                if taken {
                                    // taken path
                                    branch_index = branch.targ_addr_branch_index.unwrap();
                                } else {
                                    // not taken path
                                    branch_index += 1;
                                }
                                break;
                            }
                            BranchType::Return => {
                                // ret compression: if the target address of ret matches the call,
                                // it is stored as a taken bit in TNT packet
                                assert!(taken);
                                let (target_ip, target_branch_index) =
                                    call_stack.pop_back().unwrap();

                                record_indirect(&mut output_trace, branch, target_ip)?;

                                // go to target address
                                branch_index = target_branch_index;

                                break;
                            }
                            BranchType::DirectCall => {
                                // add to call stack
                                // branch_index+1: the first branch on the fallthrough path
                                call_stack.push_back((branch.fall_addr(), branch_index + 1));
                                // handle call stack overflow
                                while call_stack.len() > 64 {
                                    call_stack.pop_front();
                                }

                                record_direct(
                                    &mut output_trace,
                                    branch,
                                    &mut output_branch_indices[branch_index],
                                    true,
                                )?;

                                // go to target address
                                branch_index = branch.targ_addr_branch_index.unwrap();
                            }
                            BranchType::DirectJump => {
                                record_direct(
                                    &mut output_trace,
                                    branch,
                                    &mut output_branch_indices[branch_index],
                                    true,
                                )?;

                                // go to target address
                                branch_index = branch.targ_addr_branch_index.unwrap();
                            }
                            _ => unimplemented!(
                                "Unhandled branch {:x?} when handling packet {:x?}",
                                branch,
                                tnt
                            ),
                        }
                    }
                }
            }
            Packet::TIP(tip) => {
                // wait until we found the indirect branch
                loop {
                    let branch = &branches[branch_index];

                    match branch.branch_type {
                        BranchType::Return => {
                            record_indirect(&mut output_trace, branch, tip.target_ip)?;

                            // find branch @ target address
                            branch_index = find_branch_by_pc(&branches, tip.target_ip);

                            // maintain call stack
                            call_stack.pop_front();
                            break;
                        }
                        BranchType::DirectCall => {
                            record_direct(
                                &mut output_trace,
                                branch,
                                &mut output_branch_indices[branch_index],
                                true,
                            )?;

                            // add to call stack
                            // branch_index+1: the first branch on the fallthrough path
                            call_stack.push_back((branch.fall_addr(), branch_index + 1));
                            // handle call stack overflow
                            while call_stack.len() > 64 {
                                call_stack.pop_front();
                            }

                            // go to target address
                            branch_index = branch.targ_addr_branch_index.unwrap();
                        }
                        BranchType::IndirectCall => {
                            record_indirect(&mut output_trace, branch, tip.target_ip)?;

                            // add to call stack
                            // branch_index+1: the first branch on the fallthrough path
                            call_stack.push_back((branch.fall_addr(), branch_index + 1));
                            // handle call stack overflow
                            while call_stack.len() > 64 {
                                call_stack.pop_front();
                            }

                            // find branch @ target address
                            branch_index = find_branch_by_pc(&branches, tip.target_ip);
                            break;
                        }
                        BranchType::IndirectJump => {
                            record_indirect(&mut output_trace, branch, tip.target_ip)?;

                            // find branch @ target address
                            branch_index = find_branch_by_pc(&branches, tip.target_ip);
                            break;
                        }
                        BranchType::DirectJump => {
                            record_direct(
                                &mut output_trace,
                                branch,
                                &mut output_branch_indices[branch_index],
                                true,
                            )?;

                            // go to target address
                            branch_index = branch.targ_addr_branch_index.unwrap();
                        }
                        _ => unimplemented!(
                            "Unhandled branch {:x?} when handling packet {:x?}",
                            branch,
                            tip
                        ),
                    }
                }
            }
        }
    }
    // collect images
    output_trace.images = images;

    println!(
        "Got {} branches, {} entries and {} images in output trace",
        output_trace.branches.len(),
        output_trace.num_entries,
        output_trace.images.len()
    );
    output_trace.finish()?;

    Ok(())
}
