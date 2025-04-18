//! Display info and statistics of trace file
use cbp_experiments::{
    Branch, BranchType, TraceFileDecoder, create_inst_index_mapping_from_images, get_inst_index,
    get_tqdm_style,
};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use log::{Level, log_enabled, trace};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    #[arg(short, long)]
    trace_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchInfo {
    execution_count: u64,
    taken_count: u64,
    inst_addr_index: u64,
    targ_addr_index: u64,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Cli::parse();
    let content = std::fs::read(args.trace_path)?;
    // parse trace file
    let file = TraceFileDecoder::open(&content);
    println!(
        "Got {} branches, {}({:.2e}, {:.2} bit/entry) entries and {} images",
        file.num_brs,
        file.num_entries,
        file.num_entries,
        content.len() as f64 * 8.0 / file.num_entries as f64,
        file.num_images
    );

    println!("Loaded images:");
    for image in file.images.iter() {
        println!(
            "Image {} loaded to 0x{:x}",
            image.get_filename()?,
            image.start
        );
    }

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
    let mapping: HashMap<u64, u64> = create_inst_index_mapping_from_images(file.images)?;

    let mut branch_infos = vec![BranchInfo::default(); file.num_brs];

    // preprocess instruction indices for all branches
    for (i, branch) in file.branches.iter().enumerate() {
        branch_infos[i].inst_addr_index = get_inst_index(&mapping, branch.inst_addr);
        branch_infos[i].targ_addr_index = get_inst_index(&mapping, branch.targ_addr);
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
            branch_infos[br_index].taken_count += taken as u64;

            if log_enabled!(Level::Trace) {
                let pc = file.branches[br_index].inst_addr;
                let mut addr = format!("unknown:0x{:x}", pc);
                for image in file.images {
                    if pc >= image.start && pc < image.start + image.len {
                        addr =
                            format!("{}:0x{:x}", image.get_filename().unwrap(), pc - image.start);
                    }
                }
                trace!(
                    "PC = 0x{:x} ({}) {}",
                    pc,
                    addr,
                    if taken { "T" } else { "N" }
                );
            }

            // instruction counting
            if taken {
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
    println!("Executed {} instructions", instructions);

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
            file.get_addr_location(branch.inst_addr)?.cell(),
        ]);
    }
    let table = table.table().title(vec![
        "Branch PC".cell(),
        "Branch Type".cell(),
        "Execution Count".cell(),
        "Taken Rate (%)".cell(),
        "Image & Offset".cell(),
    ]);
    print_stdout(table)?;

    Ok(())
}
