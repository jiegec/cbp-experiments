//! Test branch prediction accuracy
use cbp_experiments::new_predictor;
use cbp_experiments::{
    Branch, BranchType, TraceFileDecoder, create_insn_index_mapping, get_tqdm_style,
};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    #[arg(short, long)]
    trace_path: PathBuf,

    /// Predictor name
    #[arg(short, long)]
    predictor: String,

    /// Path to executable file
    #[arg(short, long)]
    exe_path: PathBuf,

    /// Skip count in instructions
    #[arg(short, long, default_value = "0")]
    skip: usize,

    /// Warmup count in instructions
    #[arg(short, long, default_value = "0")]
    warmup: usize,

    /// Simulation count in instructions
    #[arg(short, long, default_value = "0")]
    simulate: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchInfo {
    execution_count: u64,
    taken_count: u64,
    mispred_count: u64,
    inst_addr_index: usize,
    targ_addr_index: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(args.trace_path)?;

    // parse trace file
    let file = TraceFileDecoder::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_brs, file.num_entries
    );
    println!(
        "Skip {} instructions, warmup {} instructions and simulate {} instructions",
        args.skip, args.warmup, args.simulate
    );

    let mut predictor = new_predictor(&args.predictor);
    let mut predictor_mut = predictor.as_mut().unwrap();

    // create a mapping from instruction address to instruction index for instruction counting
    let mapping = create_insn_index_mapping(&args.exe_path)?;

    let mut branch_infos = vec![BranchInfo::default(); file.num_brs];

    // preprocess instruction indices for all branches
    for (i, branch) in file.branches.iter().enumerate() {
        branch_infos[i].inst_addr_index = *mapping.get(&branch.inst_addr).unwrap();
        branch_infos[i].targ_addr_index = *mapping.get(&branch.targ_addr).unwrap();
    }

    let pbar = indicatif::ProgressBar::new(0);
    pbar.set_style(get_tqdm_style());

    let mut last_targ_addr_index = None;
    let mut instructions = 0;
    let mut first_simulate = true;
    for entries in file.entries()? {
        for entry in entries {
            let br_index = entry.get_br_index();
            let taken = entry.get_taken();

            // add instruction counting
            if taken {
                let curr_index = branch_infos[br_index].inst_addr_index;
                if let Some(last_index) = last_targ_addr_index {
                    // count instructions from last target address to the current branch address
                    assert!(curr_index >= last_index);
                    instructions += curr_index - last_index + 1;
                }
                last_targ_addr_index = Some(branch_infos[br_index].targ_addr_index);
            }

            if instructions < args.skip {
                continue;
            }

            // collect statistics
            if instructions >= args.skip + args.warmup {
                branch_infos[entry.get_br_index()].execution_count += 1;
                branch_infos[entry.get_br_index()].taken_count += entry.get_taken() as u64;
            }

            if instructions >= args.skip + args.warmup && first_simulate {
                println!("Simulation begins at instruction {}", instructions);
                first_simulate = false;
            }

            // predict or train
            let branch = &file.branches[entry.get_br_index()];
            if branch.branch_type == BranchType::ConditionalDirectJump {
                // requires prediction
                let predict = predictor_mut.as_mut().get_prediction(branch.inst_addr);
                if instructions >= args.skip + args.warmup {
                    branch_infos[entry.get_br_index()].mispred_count +=
                        (predict != entry.get_taken()) as u64;
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

            if instructions >= args.skip + args.warmup + args.simulate {
                break;
            }
        }

        if instructions < args.skip {
            pbar.set_length(args.skip as u64);
            pbar.set_position(instructions as u64);
        } else if instructions < args.skip + args.warmup {
            pbar.set_length(args.warmup as u64);
            pbar.set_position((instructions - args.skip) as u64);
        } else {
            pbar.set_length(args.simulate as u64);
            pbar.set_position((instructions - args.skip - args.warmup) as u64);
        }

        if instructions >= args.skip + args.warmup + args.simulate {
            break;
        }
    }

    pbar.finish();
    println!("Simulation ends at instruction {}", instructions);

    println!("Top branches by misprediction count:");
    let mut items: Vec<(&BranchInfo, &Branch)> = branch_infos.iter().zip(file.branches).collect();

    items.sort_by_key(|(info, _)| info.mispred_count);
    let mut table = vec![];
    for (info, branch) in items.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", branch.inst_addr).cell(),
            info.execution_count.cell(),
            info.mispred_count.cell(),
            format!(
                "{:.2}",
                info.taken_count as f64 * 100.0 / info.execution_count as f64
            )
            .cell(),
            format!(
                "{:.2}",
                info.mispred_count as f64 * 100.0 / info.execution_count as f64
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

    // find hard to predict branches:
    // 1. less than 99% prediction accuracy
    // 2. execute at least 15000 times per 30M instructions
    // 3. generate at least 1000 mispredictions per 30M instructions
    let mut h2p_execute_count = 0;
    let mut h2p_mispred_count = 0;
    let mut h2p_count = 0;
    for (info, _) in items.iter() {
        let accuracy = 1.0 - info.mispred_count as f64 * 100.0 / info.execution_count as f64;
        if accuracy >= 0.99 {
            continue;
        }

        if info.execution_count as f64 / args.simulate as f64 * 30000000.0 < 15000.0 {
            continue;
        }

        if info.mispred_count as f64 / args.simulate as f64 * 30000000.0 < 1000.0 {
            continue;
        }

        // this is a hard to predict branch
        // println!("Found hard to predict branch {:x?} {:x?}", branch, info);
        h2p_execute_count += info.execution_count;
        h2p_mispred_count += info.mispred_count;
        h2p_count += 1;
    }

    let num_cond_brs = file
        .branches
        .iter()
        .filter(|branch| branch.branch_type == BranchType::ConditionalDirectJump)
        .count();
    let num_cond_brs_executed = file
        .branches
        .iter()
        .zip(&branch_infos)
        .filter(|(branch, info)| {
            branch.branch_type == BranchType::ConditionalDirectJump && info.execution_count > 0
        })
        .count();

    println!("Overall statistics (H2P branches means hard to predict conditional branches):");
    // compute mpki
    let total_cond_execution_count: u64 = branch_infos
        .iter()
        .zip(file.branches)
        .filter(|(_, branch)| branch.branch_type == BranchType::ConditionalDirectJump)
        .map(|(info, _)| info.execution_count)
        .sum();
    let total_mispred_count: u64 = branch_infos.iter().map(|info| info.mispred_count).sum();
    println!(
        "- Number of conditional branches (total static branches): {}",
        num_cond_brs,
    );
    println!(
        "- Number of conditional branches executed at least once (static branches per slice): {}",
        num_cond_brs_executed,
    );
    println!(
        "- Conditional branch mispredictions: {}",
        total_mispred_count,
    );
    println!(
        "- Conditional branch mispredictions per kilo instructions (CMPKI): {:.2} = {} * 1000 / {}",
        total_mispred_count as f64 * 1000.0 / args.simulate as f64,
        total_mispred_count,
        args.simulate
    );
    println!(
        "- Executed conditional branches: {}",
        total_cond_execution_count,
    );
    println!(
        "- Prediction accuracy of conditional branches: {:.2}% = 1 - {} / {}",
        100.0 - total_mispred_count as f64 * 100.0 / total_cond_execution_count as f64,
        total_mispred_count,
        total_cond_execution_count
    );
    println!(
        "- Number of H2P conditional branches (static H2P branches): {}",
        h2p_count,
    );
    println!(
        "- Execution count of H2P branches (dynamic executions of H2P branches): {}",
        h2p_execute_count,
    );
    println!(
        "- Execution count per H2P branches (dynamic executions per H2P branches): {:.2} = {} / {}",
        h2p_execute_count as f64 / h2p_count as f64,
        h2p_execute_count,
        h2p_count
    );
    println!(
        "- Mispredictions due to H2P branches: {}",
        h2p_mispred_count,
    );
    println!(
        "- Prediction accuracy of H2P branches: {:.2}% = 1 - {} / {}",
        100.0 - h2p_mispred_count as f64 * 100.0 / h2p_execute_count as f64,
        h2p_mispred_count,
        h2p_execute_count
    );
    println!(
        "- Prediction accuracy of conditional branches excluding H2P branches: {:.2}% = 1 - {} / {}",
        100.0
            - (total_mispred_count - h2p_mispred_count) as f64 * 100.0
                / (total_cond_execution_count - h2p_execute_count) as f64,
        total_mispred_count - h2p_mispred_count,
        total_cond_execution_count - h2p_execute_count
    );
    println!(
        "- Conditional branch mispredictions due to H2P branches: {:.2}% = {} / {}",
        h2p_mispred_count as f64 * 100.0 / total_mispred_count as f64,
        h2p_mispred_count,
        total_mispred_count
    );
    println!(
        "- Ratio of H2P branches to all conditional branches: {:.2}% = {} / {}",
        h2p_count as f64 * 100.0 / file.num_brs as f64,
        h2p_count,
        num_cond_brs
    );

    Ok(())
}
