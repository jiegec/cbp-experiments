//! Combine simulation results of multiple SimPoint phases
use cbp_experiments::{
    Branch, BranchType, SimPointResult, SimulateResult, SimulateResultBranchInfo,
};
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::{collections::HashMap, fs::File, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to SimPoint result
    #[arg(short, long)]
    simpoint_path: PathBuf,

    /// Path to the folder containing simulation results
    #[arg(short, long)]
    result_path: PathBuf,

    /// Path to output file
    #[arg(short, long)]
    output_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    println!(
        "Loading SimPoint result from {}",
        args.simpoint_path.display()
    );
    let simpoint_result: SimPointResult =
        serde_json::from_reader(File::open(&args.simpoint_path)?)?;

    let mut combined = SimulateResult {
        trace_path: simpoint_result.trace_path.clone(),
        exe_path: simpoint_result.exe_path.clone(),
        predictor: String::new(),
        skip: 0,
        warmup: 0,
        simulate: simpoint_result.total_instructions as usize,
        branch_info: vec![],
    };

    // maintain mapping from branch to index in branch_info array
    let mut mapping: HashMap<Branch, usize> = HashMap::new();
    for (simpoint_index, phase) in simpoint_result.phases.iter().enumerate() {
        let result_file = args.result_path.join(format!(
            "{}-simpoint-{}.log",
            args.simpoint_path.file_stem().unwrap().to_str().unwrap(),
            simpoint_index
        ));
        println!("Loading simulation result from {}", result_file.display());
        let simulate_result: SimulateResult = serde_json::from_reader(File::open(&result_file)?)?;
        combined.predictor = simulate_result.predictor;

        // merge branch info
        for info in &simulate_result.branch_info {
            // since only half of the simpoint is used for simulation (the other half is used for warmup)
            // the counts should be multipled by phase.weight * 2
            let weight = phase.weight * 2;
            match mapping.get(&info.branch) {
                Some(index) => {
                    combined.branch_info[*index].execution_count += info.execution_count * weight;
                    combined.branch_info[*index].taken_count += info.taken_count * weight;
                    combined.branch_info[*index].mispred_count += info.mispred_count * weight;
                }
                None => {
                    mapping.insert(info.branch, combined.branch_info.len());
                    combined.branch_info.push(SimulateResultBranchInfo {
                        branch: info.branch,
                        execution_count: info.execution_count * weight,
                        taken_count: info.taken_count * weight,
                        mispred_count: info.mispred_count * weight,
                    });
                }
            }
        }
    }

    println!("Top branches by execution count:");
    combined
        .branch_info
        .sort_by_key(|info| info.execution_count);
    let mut table = vec![];
    for info in combined.branch_info.iter().rev().take(10) {
        table.push(vec![
            format!("0x{:08x}", info.branch.inst_addr).cell(),
            format!("{:?}", info.branch.branch_type).cell(),
            info.execution_count.cell(),
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
        "Taken Rate (%)".cell(),
        "Misprediction Rate (%)".cell(),
    ]);
    print_stdout(table)?;

    println!("Overall statistics:");
    // compute mpki
    let num_cond_brs_executed = combined
        .branch_info
        .iter()
        .filter(|info| {
            info.branch.branch_type == BranchType::ConditionalDirectJump && info.execution_count > 0
        })
        .count();
    let total_cond_execution_count: u64 = combined
        .branch_info
        .iter()
        .filter(|info| info.branch.branch_type == BranchType::ConditionalDirectJump)
        .map(|info| info.execution_count)
        .sum();
    let total_mispred_count: u64 = combined
        .branch_info
        .iter()
        .map(|info| info.mispred_count)
        .sum();
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
        total_mispred_count as f64 * 1000.0 / simpoint_result.total_instructions as f64,
        total_mispred_count,
        simpoint_result.total_instructions
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

    println!("Combined result written to {}", args.output_path.display());
    std::fs::write(args.output_path, serde_json::to_vec_pretty(&combined)?)?;

    Ok(())
}
