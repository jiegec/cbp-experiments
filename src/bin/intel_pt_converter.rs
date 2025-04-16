//! Parse Intel PT trace in perf.data and convert to our trace format

use cbp_experiments::{BranchType, Image, TraceFileEncoder, find_branches, get_tqdm_style};
use clap::Parser;
use indicatif::ProgressBar;
use log::{Level, log_enabled, trace};
use memmap::{Mmap, MmapOptions};
use object::{Object, ObjectKind, elf, read::elf::ProgramHeader};
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

pub struct PerfDataIterator {
    content: Mmap,
    pbar: ProgressBar,
    offset: usize,
    data_begin: usize,
    data_end: usize,
}

pub enum PerfDataEntry {
    /// mmap-ed image
    Image(Box<Image>),
    /// Intel PT packet
    IntelPT(Vec<Packet>),
}

impl Iterator for PerfDataIterator {
    type Item = PerfDataEntry;

    fn next(&mut self) -> Option<Self::Item> {
        // scan for more packets

        while self.offset < self.data_end {
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
                //     self.offset + event_size as usize,
                //     data_size
                // );
                self.offset += event_size as usize + data_size;
                return Some(PerfDataEntry::IntelPT(parse_intel_pt_packets(data)));
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

                self.offset += event_size as usize;
                return Some(PerfDataEntry::Image(Box::new(Image {
                    start: start - offset,
                    len: len + offset,
                    filename,
                })));
            } else {
                // not interested
                self.offset += event_size as usize;
            }

            self.pbar
                .set_position((self.offset - self.data_begin) as u64);
        }

        None
    }
}

impl PerfDataIterator {
    pub fn from<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
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

        Ok(Self {
            content,
            pbar,
            offset: data_section_offset,
            data_begin: data_section_offset,
            data_end: data_section_offset + data_section_size,
        })
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Cli::parse();

    let mut branches: Vec<BranchInfo> = vec![];

    // find the first branch that appears after or equal to the target address
    let find_branch_by_pc = |branches: &Vec<BranchInfo>, pc: u64| {
        match branches.binary_search_by_key(&pc, |info| info.inst_addr) {
            Ok(index) => {
                // exact match
                index
            }
            Err(index) => {
                // the immediate next
                assert!(
                    index < branches.len(),
                    "Failed to find branch with pc 0x{:x}",
                    pc
                );
                index
            }
        }
    };

    // starting from entrypoint, iterate branches
    // we don't know the branch index for now
    let mut branch_index = usize::MAX;
    // maintain call stack of depth 64, storing return address & next branch index of calls
    let mut call_stack: VecDeque<(u64, usize)> = VecDeque::new();

    println!("Writing to trace file at {}", args.output_path.display());
    let output_file = File::create(&args.output_path)?;
    let mut output_trace = TraceFileEncoder::open(&output_file)?;

    // Maintain branch index in output file as optimization
    let mut output_branch_indices: Vec<Option<usize>> = vec![];

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

    let trace = |output_trace: &TraceFileEncoder<'_>,
                 branches: &Vec<BranchInfo>,
                 branch_index: usize| {
        if log_enabled!(Level::Trace) {
            let pc = branches[branch_index].inst_addr;
            let mut addr = format!("unknown:0x{:x}", pc);
            for image in &output_trace.images {
                if pc >= image.start && pc < image.start + image.len {
                    addr = format!("{}:0x{:x}", image.get_filename().unwrap(), pc - image.start);
                }
            }
            trace!("PC = 0x{:x} ({})", branches[branch_index].inst_addr, addr);
        }
    };

    // interpreter for executable
    let mut interpreter = None;
    // entrypoint address
    let mut entrypoint = None;

    for entry in PerfDataIterator::from(args.trace_path)? {
        match entry {
            // parse elf, find all branches and put them in an array
            PerfDataEntry::Image(image) => {
                // parse all images before we start reconstructing control flow
                assert_eq!(branch_index, usize::MAX);

                // collect images
                let mut image_filename = image.get_filename()?;
                println!(
                    "Found image {} loaded from 0x{:x} to 0x{:x}",
                    image_filename,
                    image.start,
                    image.start + image.len
                );
                output_trace.images.push(*image);

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

                for branch in find_branches(&image_filename, load_base)? {
                    // range validation
                    assert!(
                        image.start <= branch.inst_addr
                            && branch.inst_addr < image.start + image.len
                    );

                    branches.push(BranchInfo {
                        branch_type: branch.branch_type,
                        inst_addr: branch.inst_addr,
                        inst_length: branch.inst_length,
                        targ_addr: branch.targ_addr,
                        targ_addr_branch_index: None,
                    });
                }

                // sort branches by inst address for monotonicity
                branches.sort_by_key(|branch| branch.inst_addr);

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
                for (branch, targ_addr_branch_index) in
                    branches.iter_mut().zip(targ_addr_branch_indices)
                {
                    branch.targ_addr_branch_index = targ_addr_branch_index;
                }
                output_branch_indices.resize(branches.len(), None);

                // find interpreter
                if file.kind() == ObjectKind::Dynamic {
                    // if it is a PIE, find its PT_INTERP

                    match &file {
                        object::File::Elf64(elf_file) => {
                            for segment in elf_file.elf_program_headers() {
                                if segment.p_type(elf_file.endian()) == elf::PT_INTERP {
                                    let offset = segment.p_offset(elf_file.endian());
                                    let size = segment.p_filesz(elf_file.endian());
                                    let content =
                                        &binary_data[offset as usize..(offset + size) as usize];
                                    let len = content
                                        .iter()
                                        .position(|ch| *ch == 0)
                                        .unwrap_or(content.len());

                                    let str = String::from_utf8(Vec::from(&content[..len]))?;
                                    let path = std::fs::canonicalize(&str)?;
                                    println!("Found interpreter {}", path.display());
                                    interpreter = Some(format!("{}", path.display()));
                                }
                            }
                        }
                        _ => unimplemented!("Unsupported binary type"),
                    }
                }

                if (file.kind() == ObjectKind::Executable && interpreter.is_none())
                    || interpreter == Some(image_filename)
                {
                    // case 1, statically linked executable: this executable provides the entrypoint
                    // case 2, interpreter found: this interpreter provides the entrypoint
                    let entry_pc = file.entry() + load_base;
                    entrypoint = Some(entry_pc);
                    println!("Reconstructing control from entrypoint 0x{:x}", entry_pc);
                }
            }
            PerfDataEntry::IntelPT(packets) => {
                if branch_index == usize::MAX {
                    // initialize branch index from entrypoint
                    // the branches array must not change after this
                    branch_index = find_branch_by_pc(&branches, entrypoint.unwrap());
                    trace(&output_trace, &branches, branch_index);
                }

                for packet in packets {
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
                                                branch_index =
                                                    branch.targ_addr_branch_index.unwrap();
                                            } else {
                                                // not taken path
                                                branch_index += 1;
                                            }
                                            trace(&output_trace, &branches, branch_index);
                                            break;
                                        }
                                        BranchType::Return => {
                                            // ret compression: if the target address of ret matches the call,
                                            // it is stored as a taken bit in TNT packet
                                            assert!(
                                                taken,
                                                "Got packet {:?} while expected a return {:x?}",
                                                packet, branch
                                            );
                                            let (target_ip, target_branch_index) =
                                                call_stack.pop_back().unwrap();

                                            record_indirect(&mut output_trace, branch, target_ip)?;

                                            // go to target address
                                            branch_index = target_branch_index;
                                            trace(&output_trace, &branches, branch_index);

                                            break;
                                        }
                                        BranchType::DirectCall => {
                                            // add to call stack
                                            // branch_index+1: the first branch on the fallthrough path
                                            call_stack
                                                .push_back((branch.fall_addr(), branch_index + 1));
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
                                            trace(&output_trace, &branches, branch_index);
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
                                            trace(&output_trace, &branches, branch_index);
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
                                        trace(&output_trace, &branches, branch_index);

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
                                        call_stack
                                            .push_back((branch.fall_addr(), branch_index + 1));
                                        // handle call stack overflow
                                        while call_stack.len() > 64 {
                                            call_stack.pop_front();
                                        }

                                        // go to target address
                                        branch_index = branch.targ_addr_branch_index.unwrap();
                                        trace(&output_trace, &branches, branch_index);
                                    }
                                    BranchType::IndirectCall => {
                                        record_indirect(&mut output_trace, branch, tip.target_ip)?;

                                        // add to call stack
                                        // branch_index+1: the first branch on the fallthrough path
                                        call_stack
                                            .push_back((branch.fall_addr(), branch_index + 1));
                                        // handle call stack overflow
                                        while call_stack.len() > 64 {
                                            call_stack.pop_front();
                                        }

                                        // find branch @ target address
                                        branch_index = find_branch_by_pc(&branches, tip.target_ip);
                                        trace(&output_trace, &branches, branch_index);
                                        break;
                                    }
                                    BranchType::IndirectJump => {
                                        record_indirect(&mut output_trace, branch, tip.target_ip)?;

                                        // find branch @ target address
                                        branch_index = find_branch_by_pc(&branches, tip.target_ip);
                                        trace(&output_trace, &branches, branch_index);
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
                                        trace(&output_trace, &branches, branch_index);
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
            }
        }
    }

    println!(
        "Got {} branches, {} entries and {} images in output trace",
        output_trace.branches.len(),
        output_trace.num_entries,
        output_trace.images.len()
    );
    output_trace.finish()?;

    Ok(())
}
