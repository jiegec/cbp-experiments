/// Display info and statistics of trace file
use cbp_experiments::{Branch, BranchType, TraceFile, get_tqdm_style};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,
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

    let mut branch_execution_counts = vec![0usize; file.num_brs];
    let mut branch_taken_counts = vec![0usize; file.num_brs];

    println!("Iterating entries");
    let pbar = indicatif::ProgressBar::new(file.num_entries as u64);
    pbar.set_style(get_tqdm_style());
    for entries in file.entries()? {
        for entry in entries {
            branch_execution_counts[entry.get_br_index()] += 1;
            branch_taken_counts[entry.get_br_index()] += entry.get_taken() as usize;
        }

        pbar.inc(entries.len() as u64);
    }
    pbar.finish();

    println!("Top branches by execution count:");
    let mut items: Vec<((&usize, &usize), &Branch)> = branch_execution_counts
        .iter()
        .zip(branch_taken_counts.iter())
        .zip(file.branches)
        .collect();

    items.sort_by_key(|((execution_count, _), _)| **execution_count);
    let mut table = vec![];
    for ((execution_count, taken_count), branch) in items.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", branch.inst_addr).cell(),
            format!("{:?}", branch.branch_type).cell(),
            execution_count.cell(),
            format!(
                "{:.2}",
                **taken_count as f64 * 100.0 / **execution_count as f64
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
