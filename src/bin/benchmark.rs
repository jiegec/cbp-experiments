//! Operations on predefined benchmarks
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::path::PathBuf;

// benchmark folder structure
// benchmarks/
// \- config-name
//    |- config.json
//    \- traces
//       \- tracer-name
//          \- benchmark-name-0.log

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, ValueEnum)]
enum Tracer {
    /// Pin tracer
    Pin,
    /// DynamoRIO tracer
    #[clap(name = "dynamorio")]
    DynamoRIO,
    /// Intel PT tracer
    IntelPT,
}

#[derive(Subcommand)]
enum Commands {
    /// Record trace using tracers
    Record {
        /// Which tracer to use
        tracer: Tracer,

        /// Benchmark folder name under benchmarks/
        folder: PathBuf,
    },
}

#[derive(Clone, Deserialize)]
struct Command {
    /// Command line args
    args: String,
}

#[derive(Clone, Deserialize)]
struct Benchmark {
    /// Benchmark name
    name: String,
    /// Path to its executable
    executable: String,
    /// It may contain multiple commands to run
    commands: Vec<Command>,
}

#[derive(Clone, Deserialize)]
struct Config {
    benchmarks: Vec<Benchmark>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match &args.command {
        Commands::Record { tracer, folder } => {
            let tracer_possible_value = tracer.to_possible_value().unwrap();
            let tracer_name = tracer_possible_value.get_name();
            let config: Config = serde_json::from_slice(&std::fs::read(
                PathBuf::from("benchmarks").join(folder).join("config.json"),
            )?)?;

            for benchmark in &config.benchmarks {
                for (command_index, command) in benchmark.commands.iter().enumerate() {
                    // run: "{benchmark.executable} {command.args}"
                    // generate trace file at "benchmarks/{folder}/traces/{tracer}/{benchmark.name}-{command_index}.log"
                    let dir = PathBuf::from("benchmarks")
                        .join(folder)
                        .join("traces")
                        .join(tracer_name);
                    std::fs::create_dir_all(&dir)?;
                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!(
                        "Generate trace file {} using {}: {} {}",
                        trace_file.display(),
                        tracer_name,
                        benchmark.executable,
                        command.args
                    );
                    match tracer {
                        Tracer::Pin => {
                            let args = format!(
                                "time ~/prefix/pin/pin -t tracers/pin/obj-intel64/brtrace.so -o {} -- {} {}",
                                trace_file.display(),
                                benchmark.executable,
                                command.args
                            );
                            println!("Running {}", args);
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .status()?;
                            assert!(result.success());
                        }
                        Tracer::DynamoRIO => {
                            let args = format!(
                                "time ~/prefix/dynamorio/bin64/drrun -c tracers/dynamorio/build/libbrtrace.so {} -- {} {}",
                                trace_file.display(),
                                benchmark.executable,
                                command.args
                            );
                            println!("Running {}", args);
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .status()?;
                            assert!(result.success());
                        }
                        Tracer::IntelPT => {
                            // record intel pt
                            let perf_data_file =
                                dir.join(format!("{}-{}-perf.log", benchmark.name, command_index));
                            let args = format!(
                                "time numactl -C 0 perf record -e intel_pt//u  -o {} -- {} {}",
                                perf_data_file.display(),
                                benchmark.executable,
                                command.args
                            );
                            println!("Running {}", args);
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .status()?;
                            assert!(result.success());

                            // conversion
                            let args = format!(
                                "time target/release/intel_pt {} {} {}",
                                perf_data_file.display(),
                                benchmark.executable,
                                trace_file.display(),
                            );
                            println!("Running {}", args);
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .status()?;
                            assert!(result.success());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
