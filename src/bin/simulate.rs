//! Test branch prediction accuracy
use cbp_experiments::{
    Branch, BranchType, ImageWithoutData, TraceFileDecoder, create_inst_index_mapping_from_images,
    get_inst_index, get_tqdm_style, is_indirect, new_indirect_branch_predictor,
};
use cbp_experiments::{SimulateResult, SimulateResultBranchInfo, new_conditional_branch_predictor};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    #[arg(short, long)]
    trace_path: PathBuf,

    /// Conditional branch predictor name
    #[arg(short, long)]
    conditional_branch_predictor: String,

    /// Indirect branch predictor name
    #[arg(short, long)]
    indirect_branch_predictor: String,

    /// Skip count in instructions
    #[arg(short, long, default_value = "0")]
    skip: u64,

    /// Warmup count in instructions
    #[arg(short, long, default_value = "0")]
    warmup: u64,

    /// Simulation count in instructions
    #[arg(short, long, default_value = "0")]
    simulate: u64,

    /// Path to result json
    #[arg(short, long)]
    output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct BranchInfo {
    branch_type: BranchType,
    execution_count: u64,
    taken_count: u64,
    mispred_count: u64,
    inst_addr_index: u64,
    targ_addr_index: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(&args.trace_path)?;

    // parse trace file
    let file = TraceFileDecoder::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_branches, file.num_entries
    );
    println!(
        "Skip {} instructions, warmup {} instructions and simulate {} instructions",
        args.skip, args.warmup, args.simulate
    );

    let mut conditional_branch_predictor =
        new_conditional_branch_predictor(&args.conditional_branch_predictor);

    let mut indirect_branch_predictor =
        new_indirect_branch_predictor(&args.indirect_branch_predictor);
    let mut indirect_branch_predictor_mut = indirect_branch_predictor.as_mut().unwrap();

    // create a mapping from instruction address to instruction index for instruction counting
    let file_images = file.get_images()?;
    let mapping = create_inst_index_mapping_from_images(&file_images)?;

    let mut branch_infos = vec![];

    // preprocess instruction indices for all branches
    for branch in file.branches {
        branch_infos.push(BranchInfo {
            branch_type: branch.branch_type,
            execution_count: 0,
            taken_count: 0,
            mispred_count: 0,
            inst_addr_index: get_inst_index(&mapping, branch.inst_addr),
            targ_addr_index: get_inst_index(&mapping, branch.targ_addr),
        });
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

            let branch = &file.branches[entry.get_br_index()];

            // predict or train conditional branch predictor
            if branch.branch_type == BranchType::ConditionalDirectJump {
                // requires prediction
                let predict =
                    conditional_branch_predictor.predict(branch.inst_addr, entry.get_taken());
                if instructions >= args.skip + args.warmup {
                    branch_infos[entry.get_br_index()].mispred_count +=
                        (predict != entry.get_taken()) as u64;
                }

                // update
                conditional_branch_predictor.update(
                    branch.inst_addr,
                    branch.branch_type,
                    entry.get_taken(),
                    predict,
                    branch.targ_addr,
                );
            } else {
                // update
                conditional_branch_predictor.update_others(
                    branch.inst_addr,
                    branch.branch_type,
                    true,
                    branch.targ_addr,
                );
            }

            // predict or train indirect branch predictor
            if is_indirect(branch.branch_type) {
                // requires prediction
                let predict = indirect_branch_predictor_mut
                    .as_mut()
                    .get_indirect_branch_prediction(
                        branch.inst_addr,
                        branch.branch_type,
                        branch.targ_addr,
                    );
                if instructions >= args.skip + args.warmup {
                    branch_infos[entry.get_br_index()].mispred_count +=
                        (predict != branch.targ_addr) as u64;
                }

                // update
                indirect_branch_predictor_mut
                    .as_mut()
                    .update_indirect_branch_predictor(
                        branch.inst_addr,
                        branch.branch_type,
                        entry.get_taken(),
                        branch.targ_addr,
                    );
            } else {
                // update
                indirect_branch_predictor_mut
                    .as_mut()
                    .update_indirect_branch_predictor(
                        branch.inst_addr,
                        branch.branch_type,
                        entry.get_taken(),
                        branch.targ_addr,
                    );
            }

            if instructions >= args.skip + args.warmup + args.simulate {
                break;
            }
        }

        if instructions < args.skip {
            pbar.set_length(args.skip);
            pbar.set_position(instructions);
        } else if instructions < args.skip + args.warmup {
            pbar.set_length(args.warmup);
            pbar.set_position(instructions - args.skip);
        } else {
            pbar.set_length(args.simulate);
            pbar.set_position(instructions - args.skip - args.warmup);
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

    println!("Statistics:");
    let num_cond_brs = file
        .branches
        .iter()
        .filter(|branch| branch.branch_type == BranchType::ConditionalDirectJump)
        .count();
    println!(
        "- Number of conditional branches (total static branches): {}",
        num_cond_brs,
    );

    let num_cond_brs_executed = file
        .branches
        .iter()
        .zip(&branch_infos)
        .filter(|(branch, info)| {
            branch.branch_type == BranchType::ConditionalDirectJump && info.execution_count > 0
        })
        .count();
    println!(
        "- Number of conditional branches executed at least once (static branches per slice): {}",
        num_cond_brs_executed,
    );

    let total_mispred_count: u64 = branch_infos.iter().map(|info| info.mispred_count).sum();
    println!("- Total branch mispredictions: {}", total_mispred_count);

    // compute mpki
    let total_br_execution_count: u64 = branch_infos
        .iter()
        .zip(file.branches)
        .map(|(info, _)| info.execution_count)
        .sum();
    let total_cond_execution_count: u64 = branch_infos
        .iter()
        .zip(file.branches)
        .filter(|(_, branch)| branch.branch_type == BranchType::ConditionalDirectJump)
        .map(|(info, _)| info.execution_count)
        .sum();
    let total_cond_mispred_count: u64 = branch_infos
        .iter()
        .filter(|info| info.branch_type == BranchType::ConditionalDirectJump)
        .map(|info| info.mispred_count)
        .sum();
    println!(
        "- Conditional branch mispredictions: {}",
        total_cond_mispred_count,
    );
    let cmpki = total_cond_mispred_count as f64 * 1000.0 / args.simulate as f64;
    println!(
        "- Conditional branch mispredictions per kilo instructions (CMPKI): {:.2} = {} * 1000 / {}",
        cmpki, total_cond_mispred_count, args.simulate
    );
    println!(
        "- Runtime executions of branches: {}",
        total_br_execution_count,
    );
    println!(
        "- Runtime executions of conditional branches: {}",
        total_cond_execution_count,
    );
    let cond_branch_prediction_accuracy =
        100.0 - total_cond_mispred_count as f64 * 100.0 / total_cond_execution_count as f64;
    println!(
        "- Prediction accuracy of conditional branches: {:.2}% = 1 - {} / {}",
        cond_branch_prediction_accuracy, total_cond_mispred_count, total_cond_execution_count
    );

    // indirect branch prediction
    let total_indirect_execution_count: u64 = branch_infos
        .iter()
        .zip(file.branches)
        .filter(|(_, branch)| is_indirect(branch.branch_type))
        .map(|(info, _)| info.execution_count)
        .sum();
    let total_indirect_mispred_count: u64 = branch_infos
        .iter()
        .filter(|info| is_indirect(info.branch_type))
        .map(|info| info.mispred_count)
        .sum();
    println!(
        "- Indirect branch mispredictions: {}",
        total_indirect_mispred_count,
    );
    let impki = total_indirect_mispred_count as f64 * 1000.0 / args.simulate as f64;
    println!(
        "- Indirect branch mispredictions per kilo instructions (IMPKI): {:.2} = {} * 1000 / {}",
        impki, total_indirect_mispred_count, args.simulate
    );
    let indirect_branch_prediction_accuracy =
        100.0 - total_indirect_mispred_count as f64 * 100.0 / total_indirect_execution_count as f64;
    println!(
        "- Prediction accuracy of Indirect branches: {:.2}% = 1 - {} / {}",
        indirect_branch_prediction_accuracy,
        total_indirect_mispred_count,
        total_indirect_execution_count
    );

    if let Some(output_path) = &args.output_path {
        let mut images = vec![];
        for image in file_images {
            images.push(ImageWithoutData {
                start: image.start,
                len: image.len,
                filename: image.filename,
            });
        }

        let mut result = SimulateResult {
            trace_path: Some(args.trace_path.clone()),
            conditional_branch_predictor: args.conditional_branch_predictor.clone(),
            indirect_branch_predictor: args.indirect_branch_predictor.clone(),
            images,
            skip: args.skip,
            warmup: args.warmup,
            simulate: args.simulate,
            branch_info: vec![],
            total_mispred_count,
            total_br_execution_count,
            total_cond_execution_count,
            cmpki,
            // handle NaN
            cond_branch_prediction_accuracy: Some(cond_branch_prediction_accuracy),
            impki,
            // handle NaN
            indirect_branch_prediction_accuracy: Some(indirect_branch_prediction_accuracy),
        };
        for (info, branch) in &items {
            if info.execution_count > 0 {
                result.branch_info.push(SimulateResultBranchInfo {
                    branch: **branch,
                    execution_count: info.execution_count,
                    taken_count: info.taken_count,
                    mispred_count: info.mispred_count,
                });
            }
        }

        println!("Result written to {}", output_path.display());
        std::fs::write(output_path, serde_json::to_vec(&result)?)?;
    }

    Ok(())
}
