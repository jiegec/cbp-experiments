//! Operations on predefined benchmarks
use cbp_experiments::{
    SimPointResult, get_config_path, get_simpoint_dir, get_simulate_dir, get_trace_dir,
};
use chrono::Local;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::{
    fs::{File, create_dir_all},
    path::PathBuf,
};

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
        #[arg(short, long)]
        config_name: PathBuf,

        /// Which tracer to use
        #[arg(short, long)]
        tracer: Tracer,
    },
    /// Display trace info
    Info {
        /// Which tracer to use, default to use the final one
        #[arg(short, long)]
        tracer: Option<Tracer>,

        /// Benchmark config name
        #[arg(short, long)]
        config_name: PathBuf,
    },
    /// Run SimPoint methodology
    #[clap(name = "simpoint")]
    SimPoint {
        /// Which tracer to use, default to use the final one
        #[arg(short, long)]
        tracer: Option<Tracer>,

        /// Benchmark config name
        #[arg(short, long)]
        config_name: PathBuf,

        /// SimPoint slice size in instructions
        #[arg(short, long)]
        size: u64,
    },
    /// Simulate branch prediction
    Simulate {
        /// Benchmark config name
        #[arg(short, long)]
        config_name: PathBuf,

        /// Predictor name
        #[arg(short, long)]
        predictor: String,
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

fn get_tracer_name(tracer: &Option<Tracer>) -> String {
    match tracer {
        Some(tracer) => {
            let tracer_possible_value = tracer.to_possible_value().unwrap();
            let tracer_name = tracer_possible_value.get_name();
            tracer_name.to_string()
        }
        None => "final".to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // in case we have updated the source code
    let sh_args = "cargo build --release";
    println!("Running {}", sh_args);
    let result = std::process::Command::new("sh")
        .arg("-c")
        .arg(sh_args)
        .status()?;
    assert!(result.success());

    match &args.command {
        Commands::Record {
            tracer,
            config_name,
        } => {
            let tracer_possible_value = tracer.to_possible_value().unwrap();
            let tracer_name = tracer_possible_value.get_name();
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            for benchmark in &config.benchmarks {
                for (command_index, command) in benchmark.commands.iter().enumerate() {
                    // run: "{benchmark.executable} {command.args}"
                    // generate trace file at "{trace_dir}/{benchmark.name}-{command_index}.log"
                    let dir = get_trace_dir(config_name, tracer_name);
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
                                "time target/release/intel_pt_converter --trace-path {} --exe-path {} --output-path {}",
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
                    let dir = get_trace_dir(config_name, "final");
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
            let tracer_name = get_tracer_name(tracer);
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            for benchmark in &config.benchmarks {
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // trace file at "{trace_dir}/{benchmark.name}-{command_index}.log"
                    let dir = get_trace_dir(config_name, &tracer_name);
                    std::fs::create_dir_all(&dir)?;

                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!("Displaying info for {}", trace_file.display());

                    let args = format!(
                        "target/release/trace_info --trace-path {} --exe-path {}",
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
        Commands::SimPoint {
            tracer,
            config_name,
            size,
        } => {
            let tracer_name = get_tracer_name(tracer);
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            for benchmark in &config.benchmarks {
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // trace file at "{trace_dir}/{benchmark.name}-{command_index}.log"
                    let dir = get_trace_dir(config_name, &tracer_name);
                    std::fs::create_dir_all(&dir)?;

                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!("Running SimPoint on {}", trace_file.display());

                    // output prefix: "{simpoint_dir}/{benchmark.name}-{command_index}"
                    let dir = get_simpoint_dir(config_name);
                    std::fs::create_dir_all(&dir)?;
                    let output_prefix = dir.join(format!("{}-{}", benchmark.name, command_index));

                    let args = format!(
                        "target/release/simpoint --trace-path {} --exe-path {} --size {} --output-prefix {}",
                        trace_file.display(),
                        benchmark.executable,
                        size,
                        output_prefix.display()
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
        Commands::Simulate {
            config_name,
            predictor,
        } => {
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            // simulation result under "{simulate_dir}/"
            let simulate_dir = get_simulate_dir(
                config_name,
                &Local::now().format("%Y%m%d-%H%M%S").to_string(),
                predictor,
            );
            create_dir_all(&simulate_dir)?;

            for benchmark in &config.benchmarks {
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // simpoint result at "{simpoint_dir}/{benchmark.name}-{command_index}.json"
                    let dir = get_simpoint_dir(config_name);

                    let simpoint_result_path =
                        dir.join(format!("{}-{}.json", benchmark.name, command_index));
                    println!(
                        "Parsing SimPoint result at {}",
                        simpoint_result_path.display()
                    );

                    let simpoint_config: SimPointResult =
                        serde_json::from_reader(File::open(&simpoint_result_path)?)?;

                    // simulate on each simpoint phase
                    for (simpoint_index, _phase) in simpoint_config.phases.iter().enumerate() {
                        // trace file at "{simpoint_dir}/{benchmark.name}-{command_index}-simpoint-{simpoint_index}.log"
                        let trace_dir = get_simpoint_dir(config_name);

                        let trace_file = trace_dir.join(format!(
                            "{}-{}-simpoint-{}.log",
                            benchmark.name, command_index, simpoint_index
                        ));

                        // simulation result at "{simulate_dir}/{benchmark.name}-{command_index}-simpoint-{simpoint_index}.log"
                        let output_file = simulate_dir.join(format!(
                            "{}-{}-simpoint-{}.log",
                            benchmark.name, command_index, simpoint_index
                        ));
                        let args = format!(
                            "target/release/simulate --trace-path {} --predictor {} --exe-path {} --skip 0 --warmup {} --simulate {} --output-path {}",
                            trace_file.display(),
                            predictor,
                            benchmark.executable,
                            // half for warmup, half for simulate
                            simpoint_config.size / 2,
                            simpoint_config.size / 2,
                            output_file.display()
                        );
                        println!("Running {}", args);
                        let result = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(args)
                            .status()?;
                        assert!(result.success());
                    }

                    // combine results of simulations
                    // combined result at "{simulate_dir}/{benchmark.name}-{command_index}.log"
                    let output_file =
                        simulate_dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    let args = format!(
                        "target/release/combine --output-path {} simpoint --simpoint-path {} --result-path {}",
                        output_file.display(),
                        get_simpoint_dir(config_name)
                            .join(format!("{}-{}.json", benchmark.name, command_index))
                            .display(),
                        simulate_dir.display(),
                    );
                    println!("Running {}", args);
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(args)
                        .status()?;
                    assert!(result.success());
                }

                // combine results of different commands
                // combined result at "{simulate_dir}/{benchmark.name}.log"
                let output_file = simulate_dir.join(format!("{}.log", benchmark.name));
                let mut paths = vec![];
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // combined simpoint result at "{simulate_dir}/{benchmark.name}-{command_index}.log"
                    let output_file =
                        simulate_dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    paths.push("--command-paths".to_string());
                    paths.push(format!("{}", output_file.display()));
                }
                let args = format!(
                    "target/release/combine --output-path {} command {}",
                    output_file.display(),
                    paths.join(" ")
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

    Ok(())
}
