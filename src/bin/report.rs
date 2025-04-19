//! Combine simulation results of multiple SimPoint phases
use cbp_experiments::SimulateResult;
use clap::Parser;
use cli_table::{Cell, Table, print_stdout};
use std::{fs::File, path::PathBuf};

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
    let mut cmpkis = vec![];
    for input_file in &args.simulate_path {
        println!("Loading simulation result from {}", input_file.display());
        let simulate_result: SimulateResult = serde_json::from_reader(File::open(&input_file)?)?;
        cmpkis.push(simulate_result.cmpki);

        table.push(vec![
            input_file.file_stem().unwrap().to_str().unwrap().cell(),
            format!("{:.4}", simulate_result.cmpki).cell(),
        ]);
    }

    table.push(vec![
        "Average".cell(),
        format!("{:.4}", cmpkis.iter().sum::<f64>() / cmpkis.len() as f64).cell(),
    ]);

    let table = table
        .table()
        .title(vec!["Benchmark".cell(), "CMPKI".cell()]);
    print_stdout(table)?;

    Ok(())
}
