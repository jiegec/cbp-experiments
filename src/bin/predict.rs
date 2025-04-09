/// Test branch prediction accuracy
use cbp_experiments::ffi::new_predictor;
use cbp_experiments::{Branch, BranchType, TraceFile, create_insn_index_mapping, get_tqdm_style};
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

    /// Path to ELF file
    elf: PathBuf,

    /// Skip count in instructions
    #[arg(default_value = "0")]
    skip: usize,

    /// Warmup count in instructions
    #[arg(default_value = "0")]
    warmup: usize,

    /// Simulation count in instructions
    #[arg(default_value = "0")]
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
    let content = std::fs::read(args.trace)?;

    // parse trace file
    let file = TraceFile::open(&content);
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
    let mapping = create_insn_index_mapping(&args.elf)?;

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

            if instructions <= args.skip {
                continue;
            }

            // collect statistics
            if instructions > args.skip + args.warmup {
                branch_infos[entry.get_br_index()].execution_count += 1;
                branch_infos[entry.get_br_index()].taken_count += entry.get_taken() as u64;
            }

            // predict or train
            let branch = &file.branches[entry.get_br_index()];
            if branch.branch_type == BranchType::ConditionalDirectJump {
                // requires prediction
                let predict = predictor_mut.as_mut().get_prediction(branch.inst_addr);
                if instructions > args.skip + args.warmup {
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
        }

        if instructions <= args.skip {
            pbar.set_length(args.skip as u64);
            pbar.set_position(instructions as u64);
        } else if instructions <= args.skip + args.warmup {
            pbar.set_length(args.warmup as u64);
            pbar.set_position((instructions - args.skip) as u64);
        } else {
            pbar.set_length(args.simulate as u64);
            pbar.set_position((instructions - args.skip - args.warmup) as u64);
        }

        if instructions > args.skip + args.warmup + args.simulate {
            break;
        }
    }

    pbar.finish();

    // compute mpki
    let total_mispred_count: u64 = branch_infos.iter().map(|info| info.mispred_count).sum();
    println!(
        "MPKI: {:.2} = {} * 1000 / {}",
        total_mispred_count as f64 * 1000.0 / args.simulate as f64,
        total_mispred_count,
        args.simulate
    );

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

    Ok(())
}
