//! Combine simulation results of multiple SimPoint phases
use cbp_experiments::{
    Branch, BranchType, ParsedImage, SimPointResult, SimulateResult, SimulateResultBranchInfo,
};
use clap::{Parser, Subcommand};
use cli_table::{Cell, Table, print_stdout};
use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to output file
    #[arg(short, long)]
    output_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Combine result of SimPoint phases
    #[clap(name = "simpoint")]
    SimPoint {
        /// Path to SimPoint result
        #[arg(short, long)]
        simpoint_path: PathBuf,

        /// Path to the folder containing simulation results
        #[arg(short, long)]
        result_path: PathBuf,
    },
    /// Combine result of different commands
    Command {
        /// Path to command results
        #[arg(short, long)]
        command_paths: Vec<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // combined result
    let mut branch_info: Vec<SimulateResultBranchInfo> = vec![];
    let mut predictor = String::new();
    let mut images: Vec<ParsedImage> = vec![];
    let trace_path: Option<PathBuf>;

    // tuple of (input file, weight)
    let mut input_files: Vec<(PathBuf, u64)> = vec![];
    let mut override_total_instructions = None;
    match &args.command {
        Commands::SimPoint {
            simpoint_path,
            result_path,
        } => {
            println!("Loading SimPoint result from {}", simpoint_path.display());
            let simpoint_result: SimPointResult =
                serde_json::from_reader(BufReader::new(File::open(simpoint_path)?))?;

            for (simpoint_index, phase) in simpoint_result.phases.iter().enumerate() {
                let result_file = result_path.join(format!(
                    "{}-simpoint-{}.log",
                    simpoint_path.file_stem().unwrap().to_str().unwrap(),
                    simpoint_index
                ));

                // since only half of the simpoint is used for simulation (the other half is used for warmup)
                // the counts should be multipled by phase.weight * 2
                let weight = phase.weight * 2;
                input_files.push((result_file, weight));
            }

            // use the total instructions count from simpoint result
            override_total_instructions = Some(simpoint_result.total_instructions);
            trace_path = Some(simpoint_result.trace_path.clone());
        }
        Commands::Command { command_paths } => {
            // all command results have weight of 1
            for command_path in command_paths {
                input_files.push((command_path.clone(), 1));
            }
            trace_path = None;
        }
    }

    // maintain mapping from branch to index in branch_info array
    let mut mapping: HashMap<Branch, usize> = HashMap::new();
    let mut total_instructions = 0;
    for (input_file, weight) in input_files {
        println!("Loading simulation result from {}", input_file.display());
        let simulate_result: SimulateResult =
            serde_json::from_reader(BufReader::new(File::open(&input_file)?))?;

        // validate & save metadata
        if !predictor.is_empty() {
            assert_eq!(predictor, simulate_result.predictor);
        }
        if !images.is_empty() {
            // generate warning if images differs, it is okay if it is a dynamic library or vdso
            if images != simulate_result.images {
                println!("WARNING: Found mismatched images:");
                for image in &images {
                    println!("LEFT : {} at 0x{:x}", image.filename, image.start);
                }
                for image in &simulate_result.images {
                    println!("RIGHT: {} at 0x{:x}", image.filename, image.start);
                }
            }
        }
        predictor = simulate_result.predictor;
        images = simulate_result.images;

        total_instructions += simulate_result.simulate;

        // merge branch info
        for info in &simulate_result.branch_info {
            match mapping.get(&info.branch) {
                Some(index) => {
                    branch_info[*index].execution_count += info.execution_count * weight;
                    branch_info[*index].taken_count += info.taken_count * weight;
                    branch_info[*index].mispred_count += info.mispred_count * weight;
                }
                None => {
                    mapping.insert(info.branch, branch_info.len());
                    branch_info.push(SimulateResultBranchInfo {
                        branch: info.branch,
                        execution_count: info.execution_count * weight,
                        taken_count: info.taken_count * weight,
                        mispred_count: info.mispred_count * weight,
                    });
                }
            }
        }
    }

    // if merging simpoint result, use the total count instead
    if let Some(instructions) = override_total_instructions {
        total_instructions = instructions;
    }

    println!("Top branches by misprediction count:");
    branch_info.sort_by_key(|info| info.mispred_count);
    let mut table = vec![];
    for info in branch_info.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", info.branch.inst_addr).cell(),
            format!("{:?}", info.branch.branch_type).cell(),
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
        "Branch Type".cell(),
        "Execution Count".cell(),
        "Misprediction Count".cell(),
        "Taken Rate (%)".cell(),
        "Misprediction Rate (%)".cell(),
    ]);
    print_stdout(table)?;

    println!("Overall statistics:");
    // compute mpki
    let num_cond_brs_executed = branch_info
        .iter()
        .filter(|info| {
            info.branch.branch_type == BranchType::ConditionalDirectJump && info.execution_count > 0
        })
        .count();
    let total_br_execution_count: u64 = branch_info.iter().map(|info| info.execution_count).sum();
    let total_cond_execution_count: u64 = branch_info
        .iter()
        .filter(|info| info.branch.branch_type == BranchType::ConditionalDirectJump)
        .map(|info| info.execution_count)
        .sum();
    let total_mispred_count: u64 = branch_info.iter().map(|info| info.mispred_count).sum();
    println!(
        "- Number of conditional branches executed at least once (static branches per slice): {}",
        num_cond_brs_executed,
    );
    println!(
        "- Conditional branch mispredictions: {}",
        total_mispred_count,
    );
    let cmpki = total_mispred_count as f64 * 1000.0 / total_instructions as f64;
    println!(
        "- Conditional branch mispredictions per kilo instructions (CMPKI): {:.2} = {} * 1000 / {}",
        cmpki, total_mispred_count, total_instructions
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
        100.0 - total_mispred_count as f64 * 100.0 / total_cond_execution_count as f64;
    println!(
        "- Prediction accuracy of conditional branches: {:.2}% = 1 - {} / {}",
        cond_branch_prediction_accuracy, total_mispred_count, total_cond_execution_count
    );

    let combined = SimulateResult {
        trace_path,
        predictor,
        images,
        skip: 0,
        warmup: 0,
        simulate: total_instructions,
        branch_info,
        total_mispred_count,
        total_br_execution_count,
        total_cond_execution_count,
        cmpki,
        // handle NaN
        cond_branch_prediction_accuracy: Some(cond_branch_prediction_accuracy),
    };

    println!("Combined result written to {}", args.output_path.display());
    std::fs::write(args.output_path, serde_json::to_vec(&combined)?)?;

    Ok(())
}
