use cbp_experiments::{Branch, BranchType, Entry};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::slice;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(args.trace)?;
    // read num_brs
    let mut tmp_u64 = [0u8; 8];
    tmp_u64.copy_from_slice(&content[content.len() - 16..content.len() - 8]);
    let num_brs = u64::from_le_bytes(tmp_u64) as usize;
    tmp_u64.copy_from_slice(&content[content.len() - 8..content.len()]);
    let num_entries = u64::from_le_bytes(tmp_u64) as usize;
    println!("Got {num_brs} branches and {num_entries} entries");

    let branches: &[Branch] = unsafe {
        slice::from_raw_parts(
            &content[content.len() - 16 - std::mem::size_of::<Branch>() * num_brs as usize]
                as *const u8 as *const Branch,
            num_brs as usize,
        )
    };

    let compressed_entries: &[u8] =
        &content[0..content.len() - 16 - std::mem::size_of::<Branch>() * num_brs as usize];
    let cursor = Cursor::new(compressed_entries);
    let mut decoder = zstd::stream::read::Decoder::new(cursor)?;

    let mut branch_type_counts = [0usize; BranchType::Invalid as usize];
    for branch in branches {
        branch_type_counts[branch.branch_type as usize] += 1;
    }

    println!("Branch counts:");
    println!(
        "- direct jump: {}",
        branch_type_counts[BranchType::DirectJump as usize]
    );
    println!(
        "- indirect jump: {}",
        branch_type_counts[BranchType::IndirectJump as usize]
    );
    println!(
        "- direct call: {}",
        branch_type_counts[BranchType::DirectCall as usize]
    );
    println!(
        "- indirect call: {}",
        branch_type_counts[BranchType::IndirectCall as usize]
    );
    println!(
        "- return: {}",
        branch_type_counts[BranchType::Return as usize]
    );
    println!(
        "- conditional direct jump: {}",
        branch_type_counts[BranchType::ConditionalDirectJump as usize]
    );

    let mut branch_execution_counts = vec![0usize; num_brs];
    let mut branch_taken_counts = vec![0usize; num_brs];
    let mut pbar = tqdm::pbar(Some(num_entries));
    let mut buf = [0u8; 1024 * 256];
    loop {
        match decoder.read(&mut buf) {
            Ok(size) => {
                if size == 0 {
                    // no more data
                    break;
                }

                assert!(size % 2 == 0);
                let buf_u16: &[u16] =
                    unsafe { slice::from_raw_parts(&buf[0] as *const u8 as *const u16, size / 2) };
                for entry_raw in buf_u16 {
                    let entry = Entry(*entry_raw);
                    branch_execution_counts[entry.get_br_index()] += 1;
                    branch_taken_counts[entry.get_br_index()] += entry.get_taken() as usize;
                }
                pbar.update(buf_u16.len())?;
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to read data from zstd compressed stream: {:?}",
                    err
                ));
            }
        }
    }
    pbar.close()?;

    println!("Top branches by execution count:");
    let mut items: Vec<((&usize, &usize), &Branch)> = branch_execution_counts
        .iter()
        .zip(branch_taken_counts.iter())
        .zip(branches)
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
