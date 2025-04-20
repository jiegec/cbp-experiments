//! Combine simulation results of multiple SimPoint phases
use cbp_experiments::SimulateResult;
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Simulate log paths
    #[arg(short, long)]
    simulate_path: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let mut table = vec![];
    // compute averages for each column
    let mut columns = vec![];
    for input_file in &args.simulate_path {
        println!("Loading simulation result from {}", input_file.display());
        let simulate_result: SimulateResult =
            serde_json::from_reader(BufReader::new(File::open(&input_file)?))?;

        let total_instructions = simulate_result.simulate;

        // reproduction of paper "Branch Prediction Is Not A Solved Problem: Measurements, Opportunities, and Future Directions"
        // find hard to predict branches:
        // 1. less than 99% prediction accuracy
        // 2. execute at least 15000 times per 30M instructions
        // 3. generate at least 1000 mispredictions per 30M instructions
        let mut h2p_execute_count = 0;
        let mut h2p_mispred_count = 0;
        let mut h2p_count = 0;
        for info in simulate_result.branch_info.iter() {
            let accuracy = 1.0 - info.mispred_count as f64 * 100.0 / info.execution_count as f64;
            if accuracy >= 0.99 {
                continue;
            }

            if info.execution_count as f64 / total_instructions as f64 * 30000000.0 < 15000.0 {
                continue;
            }

            if info.mispred_count as f64 / total_instructions as f64 * 30000000.0 < 1000.0 {
                continue;
            }

            // this is a hard to predict branch
            h2p_execute_count += info.execution_count;
            h2p_mispred_count += info.mispred_count;
            h2p_count += 1;
        }

        // misprediction rate of h2p branches
        let h2p_mispred_rate =
            h2p_mispred_count as f64 * 100.0 / simulate_result.total_mispred_count as f64;
        // prediction accuracy of conditional branches
        let cond_br_acc = 100.0
            - simulate_result.total_mispred_count as f64 * 100.0
                / simulate_result.total_cond_execution_count as f64;
        // prediction accuracy of conditional branches excluding h2p branches
        let cond_br_acc_excl_h2p = 100.0
            - (simulate_result.total_mispred_count - h2p_mispred_count) as f64 * 100.0
                / (simulate_result.total_cond_execution_count - h2p_execute_count) as f64;

        columns.push((
            simulate_result.cmpki,
            h2p_count,
            h2p_mispred_rate,
            cond_br_acc,
            cond_br_acc_excl_h2p,
        ));

        table.push(vec![
            input_file.file_stem().unwrap().to_str().unwrap().cell(),
            format!("{:.4}", simulate_result.cmpki).cell(),
            format!("{:}", h2p_count).cell(),
            format!("{:.2} %", h2p_mispred_rate).cell(),
            format!("{:.2} %", cond_br_acc).cell(),
            format!("{:.2} %", cond_br_acc_excl_h2p).cell(),
        ]);
    }

    table.push(vec![
        "Average".cell(),
        format!(
            "{:.4}",
            columns.iter().map(|col| col.0).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.1}",
            columns.iter().map(|col| col.1 as f64).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.2} %",
            columns.iter().map(|col| col.2).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.2} %",
            columns.iter().map(|col| col.3).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.2} %",
            columns.iter().map(|col| col.4).sum::<f64>() / columns.len() as f64
        )
        .cell(),
    ]);

    let table = table.table().title(vec![
        "Benchmark".cell(),
        "CMPKI".cell(),
        "# Static H2P br.".cell(),
        "Misp. due to H2P br.".cell(),
        "Acc. of cond. br.".cell(),
        "Acc. of cond. br. excl. H2P".cell(),
    ]);
    print_stdout(table)?;

    Ok(())
}
