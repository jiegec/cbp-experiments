use cbp_experiments::ffi::new_predictor;
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

    /// Predictor name
    predictor: String,

    /// Warmup count
    #[arg(default_value = "0")]
    warmup: usize,

    /// Simulation count
    #[arg(default_value = "0")]
    simulation: usize,
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

    let mut predictor = new_predictor(&args.predictor);
    let mut predictor_mut = predictor.as_mut().unwrap();

    let mut branch_execution_counts = vec![0usize; num_brs];
    let mut branch_taken_counts = vec![0usize; num_brs];
    let mut branch_mispred_counts = vec![0usize; num_brs];

    let mut pbar = tqdm::pbar(Some(args.warmup + args.simulation));
    let mut buf = [0u8; 1024 * 256];
    let mut i = 0;
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
                    i += 1;
                    let entry = Entry(*entry_raw);
                    if i > args.warmup {
                        branch_execution_counts[entry.get_br_index()] += 1;
                        branch_taken_counts[entry.get_br_index()] += entry.get_taken() as usize;
                    }

                    let branch = &branches[entry.get_br_index()];
                    if branch.branch_type == BranchType::ConditionalDirectJump {
                        // requires prediction
                        let predict = predictor_mut.as_mut().get_prediction(branch.inst_addr);
                        if i > args.warmup {
                            branch_mispred_counts[entry.get_br_index()] +=
                                (predict != entry.get_taken()) as usize;
                        }

                        // update
                        predictor_mut.as_mut().update_predictor(
                            branch.inst_addr,
                            branch.branch_type,
                            entry.get_taken(),
                            predict,
                            branch.targ_addr,
                        );
                    } else {
                        // update
                        predictor_mut.as_mut().track_other_inst(
                            branch.inst_addr,
                            branch.branch_type,
                            true,
                            branch.targ_addr,
                        );
                    }
                }
                pbar.update(buf_u16.len())?;

                if i > args.warmup + args.simulation {
                    break;
                }
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

    println!("Top branches by misprediction count:");
    let mut items: Vec<(((&usize, &usize), &usize), &Branch)> = branch_execution_counts
        .iter()
        .zip(branch_taken_counts.iter())
        .zip(branch_mispred_counts.iter())
        .zip(branches)
        .collect();

    items.sort_by_key(|(((_, _), mispred_count), _)| **mispred_count);
    let mut table = vec![];
    for (((execution_count, taken_count), mispred_count), branch) in items.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", branch.inst_addr).cell(),
            execution_count.cell(),
            mispred_count.cell(),
            format!(
                "{:.2}",
                **taken_count as f64 * 100.0 / **execution_count as f64
            )
            .cell(),
            format!(
                "{:.2}",
                **mispred_count as f64 * 100.0 / **execution_count as f64
            )
            .cell(),
        ]);
    }
    let table = table.table().title(vec![
        "Branch PC".cell(),
        "Execution Count".cell(),
        "Misprediction Count".cell(),
        "Taken Rate (%)".cell(),
        "Misprediction Rate (%)".cell(),
    ]);
    print_stdout(table)?;

    Ok(())
}
