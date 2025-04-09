use cbp_experiments::ffi::new_predictor;
use cbp_experiments::{Branch, BranchType, TraceFile, get_tqdm_style};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,

    /// Predictor name
    predictor: String,

    /// Skip count
    #[arg(default_value = "0")]
    skip: usize,

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

    // parse trace file
    let file = TraceFile::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_brs, file.num_entries
    );

    let mut predictor = new_predictor(&args.predictor);
    let mut predictor_mut = predictor.as_mut().unwrap();

    let mut branch_execution_counts = vec![0usize; file.num_brs];
    let mut branch_taken_counts = vec![0usize; file.num_brs];
    let mut branch_mispred_counts = vec![0usize; file.num_brs];

    let pbar = indicatif::ProgressBar::new((args.skip + args.warmup + args.simulation) as u64);
    pbar.set_style(get_tqdm_style());
    let mut i = 0;

    for entries in file.entries()? {
        for entry in entries {
            i += 1;
            if i <= args.skip {
                continue;
            }

            if i > args.skip + args.warmup {
                branch_execution_counts[entry.get_br_index()] += 1;
                branch_taken_counts[entry.get_br_index()] += entry.get_taken() as usize;
            }

            let branch = &file.branches[entry.get_br_index()];
            if branch.branch_type == BranchType::ConditionalDirectJump {
                // requires prediction
                let predict = predictor_mut.as_mut().get_prediction(branch.inst_addr);
                if i > args.skip + args.warmup {
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

        if i <= args.skip {
            pbar.set_length(args.skip as u64);
            pbar.set_position(i as u64);
        } else if i <= args.skip + args.warmup {
            pbar.set_length(args.warmup as u64);
            pbar.set_position((i - args.skip) as u64);
        } else {
            pbar.set_length(args.simulation as u64);
            pbar.set_position((i - args.skip - args.warmup) as u64);
        }

        if i > args.skip + args.warmup + args.simulation {
            break;
        }
    }

    pbar.finish();

    println!("Top branches by misprediction count:");
    let mut items: Vec<(((&usize, &usize), &usize), &Branch)> = branch_execution_counts
        .iter()
        .zip(branch_taken_counts.iter())
        .zip(branch_mispred_counts.iter())
        .zip(file.branches)
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
