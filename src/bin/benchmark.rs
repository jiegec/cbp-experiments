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
//       \- final
//          \- benchmark-name-0.log -> ../tracer-name/benchmark-name-0.log

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
        /// Benchmark config name
        config_name: PathBuf,

        /// Which tracer to use
        tracer: Tracer,
    },
    /// Display trace info
    Info {
        /// Which tracer to use, default to use the final one
        #[arg(short, long)]
        tracer: Option<Tracer>,

        /// Benchmark config name
        config_name: PathBuf,
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
        Commands::Record {
            tracer,
            config_name,
        } => {
            let tracer_possible_value = tracer.to_possible_value().unwrap();
            let tracer_name = tracer_possible_value.get_name();
            let config: Config = serde_json::from_slice(&std::fs::read(
                PathBuf::from("benchmarks")
                    .join(config_name)
                    .join("config.json"),
            )?)?;

            for benchmark in &config.benchmarks {
                for (command_index, command) in benchmark.commands.iter().enumerate() {
                    // run: "{benchmark.executable} {command.args}"
                    // generate trace file at "benchmarks/{folder}/traces/{tracer}/{benchmark.name}-{command_index}.log"
                    let dir = PathBuf::from("benchmarks")
                        .join(config_name)
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
                                "time numactl -C 0 perf record -e intel_pt//u -o {} -- {} {}",
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
                                "time target/release/intel_pt_converter {} {} {}",
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

                    // generate trace file symlink at "benchmarks/{folder}/traces/final/{benchmark.name}-{command_index}.log"
                    let dir = PathBuf::from("benchmarks")
                        .join(config_name)
                        .join("traces")
                        .join("final");
                    std::fs::create_dir_all(&dir)?;
                    let trace_file_relative = PathBuf::from("..")
                        .join(tracer_name)
                        .join(format!("{}-{}.log", benchmark.name, command_index));
                    let final_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    std::fs::remove_file(&final_file).ok();
                    std::os::unix::fs::symlink(&trace_file_relative, &final_file)?;
                    println!(
                        "Create symlink: {} => {}",
                        final_file.display(),
                        trace_file.display(),
                    );
                }
            }
        }
        Commands::Info {
            tracer,
            config_name,
        } => {
            let tracer_name = match tracer {
                Some(tracer) => {
                    let tracer_possible_value = tracer.to_possible_value().unwrap();
                    let tracer_name = tracer_possible_value.get_name();
                    tracer_name.to_string()
                }
                None => "final".to_string(),
            };
            let config: Config = serde_json::from_slice(&std::fs::read(
                PathBuf::from("benchmarks")
                    .join(config_name)
                    .join("config.json"),
            )?)?;

            for benchmark in &config.benchmarks {
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // trace file at "benchmarks/{folder}/traces/{tracer}/{benchmark.name}-{command_index}.log"
                    let dir = PathBuf::from("benchmarks")
                        .join(config_name)
                        .join("traces")
                        .join(&tracer_name);
                    std::fs::create_dir_all(&dir)?;

                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!("Displaying info for {}", trace_file.display());

                    let args = format!(
                        "target/release/trace_info {} {}",
                        trace_file.display(),
                        benchmark.executable,
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

    Ok(())
}
