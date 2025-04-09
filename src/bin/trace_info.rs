/// Display info and statistics of trace file
use capstone::prelude::*;
use cbp_experiments::{Branch, BranchType, TraceFile, create_insn_index_mapping, get_tqdm_style};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use object::{Object, ObjectSection, SectionKind};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,

    /// Path to ELF file
    elf: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchInfo {
    execution_count: usize,
    taken_count: usize,
    inst_addr_index: usize,
    targ_addr_index: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(args.trace)?;
    // parse trace file
    let file = TraceFile::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_brs, file.num_entries
    );

    let mut branch_type_counts = [0usize; BranchType::Invalid.repr as usize];
    for branch in file.branches {
        branch_type_counts[branch.branch_type.repr as usize] += 1;
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

    // create a mapping from instruction address to instruction index for instruction counting
    let mut mapping: HashMap<u64, usize> = HashMap::new();
    if let Some(elf) = &args.elf {
        mapping = create_insn_index_mapping(elf)?;
    }

    let mut branch_infos = vec![BranchInfo::default(); file.num_brs];

    // preprocess instruction indices for all branches
    if args.elf.is_some() {
        for (i, branch) in file.branches.iter().enumerate() {
            branch_infos[i].inst_addr_index = *mapping.get(&branch.inst_addr).unwrap();
            branch_infos[i].targ_addr_index = *mapping.get(&branch.targ_addr).unwrap();
        }
    }

    println!("Iterating entries");
    let pbar = indicatif::ProgressBar::new(file.num_entries as u64);
    pbar.set_style(get_tqdm_style());
    let mut last_targ_addr_index = None;
    let mut instructions = 0;
    for entries in file.entries()? {
        for entry in entries {
            let br_index = entry.get_br_index();
            let taken = entry.get_taken();
            branch_infos[br_index].execution_count += 1;
            branch_infos[br_index].taken_count += taken as usize;

            // add instruction counting if elf is provided
            if args.elf.is_some() && taken {
                let curr_index = branch_infos[br_index].inst_addr_index;
                if let Some(last_index) = last_targ_addr_index {
                    // count instructions from last target address to the current branch address
                    assert!(curr_index >= last_index);
                    instructions += curr_index - last_index + 1;
                }
                last_targ_addr_index = Some(branch_infos[br_index].targ_addr_index);
            }
        }

        pbar.inc(entries.len() as u64);
    }
    pbar.finish();

    // accuracy: on a trimmed leela test (5% of total)
    // perf stat reported: 110979252909 instructions
    // counted: 110976357974 instructions
    // error less than 0.01%
    // slow down of counting instructions: 18s -> 38s, roughly 2x
    if args.elf.is_some() {
        println!("Executed {} instructions", instructions);
    }

    println!("Top branches by execution count:");
    let mut items: Vec<(&BranchInfo, &Branch)> = branch_infos.iter().zip(file.branches).collect();

    items.sort_by_key(|(info, _)| info.execution_count);
    let mut table = vec![];
    for (info, branch) in items.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", branch.inst_addr).cell(),
            format!("{:?}", branch.branch_type).cell(),
            info.execution_count.cell(),
            format!(
                "{:.2}",
                info.taken_count as f64 * 100.0 / info.execution_count as f64
            )
            .cell(),
        ]);
    }
    let table = table.table().title(vec![
        "Branch PC".cell(),
        "Branch Type".cell(),
        "Execution Count".cell(),
        "Taken Rate (%)".cell(),
    ]);
    print_stdout(table)?;

    Ok(())
}
