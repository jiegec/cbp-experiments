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

pub struct UniqueBranchInfo {
    /// branch
    pub branch_inst_addr: u64,
    /// statistics
    pub execution_count: u64,
    pub taken_count: u64,
    pub mispred_count: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let mut table = vec![];
    // compute averages for each column
    let mut columns = vec![];
    for input_file in &args.simulate_path {
        println!("Loading simulation result from {}", input_file.display());
        let simulate_result: SimulateResult =
            serde_json::from_reader(BufReader::new(File::open(input_file)?))?;

        columns.push((
            simulate_result.cmpki,
            simulate_result.impki,
            simulate_result.simulate,
            simulate_result.total_br_execution_count,
        ));

        table.push(vec![
            input_file.file_stem().unwrap().to_str().unwrap().cell(),
            format!("{:.4}", simulate_result.cmpki).cell(),
            format!("{:.4}", simulate_result.impki).cell(),
            format!("{:.2e}", simulate_result.simulate as f64).cell(),
            format!("{:.2e}", simulate_result.total_br_execution_count as f64).cell(),
        ]);

        // find top 10 branches sorted by mispredictions
        println!("Top branches by misprediction count:");

        // 1. sort by inst addr
        let mut items = simulate_result.branch_info.clone();
        items.sort_by_key(|info| info.branch.inst_addr);

        // 2. group by inst addr
        let mut items: Vec<UniqueBranchInfo> = items
            .as_slice()
            .chunk_by(|info1, info2| info1.branch.inst_addr == info2.branch.inst_addr)
            .map(|infos| UniqueBranchInfo {
                branch_inst_addr: infos[0].branch.inst_addr,
                execution_count: infos.iter().map(|info| info.execution_count).sum(),
                taken_count: infos.iter().map(|info| info.taken_count).sum(),
                mispred_count: infos.iter().map(|info| info.mispred_count).sum(),
            })
            .collect();

        // 3. sort by mispred count
        items.sort_by_key(|info| info.mispred_count);
        let mut table = vec![];
        for info in items.iter().rev().take(10) {
            let addr = info.branch_inst_addr;
            let mut addr_fmt = format!("unknown:0x{:x}", addr);
            let mut line_fmt = format!("unknown");
            for image in &simulate_result.images {
                if addr >= image.start && addr < image.start + image.len {
                    addr_fmt = format!(
                        "{}:0x{:x}",
                        pathdiff::diff_paths(&image.filename, std::env::current_dir()?)
                            .unwrap()
                            .display(),
                        addr - image.start
                    );
                    let file = addr2line::Loader::new(&image.filename).unwrap();
                    if let Some(location) = file.find_location(addr).unwrap() {
                        line_fmt = format!(
                            "{}:{}",
                            pathdiff::diff_paths(location.file.unwrap(), std::env::current_dir()?)
                                .unwrap()
                                .display(),
                            location.line.unwrap()
                        );
                    }
                    break;
                }
            }

            table.push(vec![
                format!("0x{:08x}", info.branch_inst_addr).cell(),
                info.execution_count.cell(),
                info.mispred_count.cell(),
                format!(
                    "{:.2}",
                    info.taken_count as f64 * 100.0 / info.execution_count as f64
                )
                .cell(),
                addr_fmt.cell(),
                line_fmt.cell(),
            ]);
        }
        let table = table.table().title(vec![
            "Br. PC".cell(),
            "Exec. count".cell(),
            "Misp. count".cell(),
            "Taken rate (%)".cell(),
            "Image & offset".cell(),
            "Source location".cell(),
        ]);
        print_stdout(table)?;
    }

    println!("Summary:");

    table.push(vec![
        "Average".cell(),
        format!(
            "{:.4}",
            columns.iter().map(|col| col.0).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.4}",
            columns.iter().map(|col| col.1).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.2e}",
            columns.iter().map(|col| col.2 as f64).sum::<f64>() / columns.len() as f64
        )
        .cell(),
        format!(
            "{:.2e}",
            columns.iter().map(|col| col.3 as f64).sum::<f64>() / columns.len() as f64
        )
        .cell(),
    ]);

    let table = table.table().title(vec![
        "Benchmark".cell(),
        "CMPKI".cell(),
        "IMPKI".cell(),
        "Insts".cell(),
        "Br insts".cell(),
    ]);
    print_stdout(table)?;

    Ok(())
}
