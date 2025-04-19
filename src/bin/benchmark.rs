//! Operations on predefined benchmarks
use anyhow::bail;
use cbp_experiments::{
    SimPointResult, get_config_path, get_simpoint_dir, get_simulate_dir, get_trace_dir,
};
use chrono::Local;
use clap::{Parser, Subcommand, ValueEnum};
use resolve_path::PathResolveExt;
use serde::Deserialize;
use std::{
    collections::VecDeque,
    fs::{File, create_dir_all},
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Instant,
};
use tempdir::TempDir;

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

        /// Run in parallel
        #[arg(short, long, default_value_t = 1)]
        parallel: usize,
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
    /// Command to run
    command: String,
}

#[derive(Clone, Deserialize)]
struct Benchmark {
    /// Benchmark name
    name: String,
    /// Path to its data folder
    data: Option<PathBuf>,
    /// Command to prepare input
    prepare: Option<String>,
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

fn run_in_shell(cmd: &str) -> anyhow::Result<()> {
    println!("Running {}", cmd);
    let time = Instant::now();
    let result = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()?;
    if !result.success() {
        bail!("Running command {} failed with {}", cmd, result);
    }
    println!("Finished running {} in {:?}", cmd, time.elapsed());
    Ok(())
}

fn run_in_parallel<T: Clone + Send + 'static>(
    args: &[T],
    parallel: usize,
    fun: impl Fn(T) -> anyhow::Result<()> + Send + Clone + 'static,
) -> anyhow::Result<()> {
    let args = VecDeque::from(args.iter().cloned().collect::<Vec<_>>());
    println!(
        "Running {} jobs in {} parallel processes",
        args.len(),
        parallel
    );
    let lock = Arc::new(Mutex::new(args));
    let mut threads = vec![];
    for _ in 0..parallel {
        let lock_clone = lock.clone();
        let fun_clone = fun.clone();
        threads.push(std::thread::spawn(move || {
            loop {
                let mut guard = lock_clone.lock().unwrap();
                match guard.pop_front() {
                    Some(arg) => {
                        drop(guard);
                        if let Err(err) = fun_clone(arg) {
                            return Err(err);
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            return Ok(());
        }));
    }

    for thread in threads {
        thread.join().unwrap()?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // in case we have updated the source code
    run_in_shell("cargo build --release")?;

    let cwd = std::env::current_dir()?;

    match &args.command {
        Commands::Record {
            tracer,
            config_name,
        } => {
            let tracer_possible_value = tracer.to_possible_value().unwrap();
            let tracer_name = tracer_possible_value.get_name();
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            if let Tracer::IntelPT = tracer {
                // check if kernel.perf_event_paranoid is set to -1
                let output = std::process::Command::new("/sbin/sysctl")
                    .arg("kernel.perf_event_paranoid")
                    .output()?;
                assert_eq!(
                    String::from_utf8_lossy(&output.stdout),
                    "kernel.perf_event_paranoid = -1\n"
                );
            }

            for benchmark in &config.benchmarks {
                // create a temporary folder to run the benchmark
                // run all commands in the same folder

                let tmp_dir = TempDir::new("cbp-experiments")?;

                // copy files under data to tmp_dir
                if let Some(data_path) = &benchmark.data {
                    let args = format!(
                        "rsync -avz --progress {}/ {}/",
                        cwd.join(data_path).display(),
                        tmp_dir.path().display()
                    );
                    run_in_shell(&args)?;
                }

                let dir = get_trace_dir(config_name, tracer_name);
                std::fs::create_dir_all(&dir)?;

                // run custom command for preparation
                if let Some(prepare) = &benchmark.prepare {
                    // resolve executable path before changing cwd
                    let mut parts = prepare.split_whitespace();
                    let executable = parts.next().unwrap();
                    let args = parts.collect::<Vec<_>>().join(" ");
                    let exe_path = executable.resolve();

                    let stdout_file = dir.join(format!("{}-prepare-stdout.log", benchmark.name,));
                    let stderr_file = dir.join(format!("{}-prepare-stderr.log", benchmark.name));
                    println!("Stdout is logged to {}", stdout_file.display());
                    println!("Stderr is logged to {}", stderr_file.display());

                    let args = format!("{} {}", exe_path.display(), args);
                    println!("Running {} under {}", args, tmp_dir.path().display());

                    let time = Instant::now();
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(args)
                        .current_dir(tmp_dir.path())
                        .stdout(File::create(stdout_file)?)
                        .stderr(File::create(stderr_file)?)
                        .status()?;
                    assert!(result.success());
                    println!("Finished in {:?}", time.elapsed());
                }

                for (command_index, command) in benchmark.commands.iter().enumerate() {
                    // run: "{benchmark.executable} {command.args}"
                    // generate trace file at "{trace_dir}/{benchmark.name}-{command_index}.log"
                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!(
                        "Generate trace file {} using {}: {}",
                        trace_file.display(),
                        tracer_name,
                        command.command
                    );

                    let stdout_file =
                        dir.join(format!("{}-{}-stdout.log", benchmark.name, command_index));
                    let stderr_file =
                        dir.join(format!("{}-{}-stderr.log", benchmark.name, command_index));
                    println!("Stdout is logged to {}", stdout_file.display());
                    println!("Stderr is logged to {}", stderr_file.display());

                    // resolve executable path before changing cwd
                    let mut parts = command.command.split_whitespace();
                    let executable = parts.next().unwrap();
                    let args = parts.collect::<Vec<_>>().join(" ");
                    let exe_path = executable.resolve();
                    match tracer {
                        Tracer::Pin => {
                            let args = format!(
                                "time ~/prefix/pin/pin -t {} -o {} -- {} {}",
                                cwd.join("tracers/pin/obj-intel64/brtrace.so").display(),
                                cwd.join(&trace_file).display(),
                                exe_path.display(),
                                args
                            );
                            println!("Running {} under {}", args, tmp_dir.path().display());

                            let time = Instant::now();
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .current_dir(tmp_dir.path())
                                .stdin(Stdio::null())
                                .stdout(File::create(stdout_file)?)
                                .stderr(File::create(stderr_file)?)
                                .status()?;
                            assert!(result.success());
                            println!("Finished in {:?}", time.elapsed());
                        }
                        Tracer::DynamoRIO => {
                            let args = format!(
                                "time ~/prefix/dynamorio/bin64/drrun -c {} {} -- {} {}",
                                cwd.join("tracers/dynamorio/build/libbrtrace.so").display(),
                                cwd.join(&trace_file).display(),
                                exe_path.display(),
                                args
                            );
                            println!("Running {} under {}", args, tmp_dir.path().display());

                            let time = Instant::now();
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .stdin(Stdio::null())
                                .stdout(File::create(stdout_file)?)
                                .stderr(File::create(stderr_file)?)
                                .current_dir(tmp_dir.path())
                                .status()?;
                            assert!(result.success());
                            println!("Finished in {:?}", time.elapsed());
                        }
                        Tracer::IntelPT => {
                            // record intel pt
                            let perf_data_file =
                                dir.join(format!("{}-{}-perf.log", benchmark.name, command_index));
                            let args = format!(
                                "time numactl -C 0 perf record -e intel_pt//u -o {} -- {} {}",
                                cwd.join(&perf_data_file).display(),
                                exe_path.display(),
                                args
                            );
                            println!("Running {} under {}", args, tmp_dir.path().display());

                            let time = Instant::now();
                            let result = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(args)
                                .stdin(Stdio::null())
                                .stdout(File::create(stdout_file)?)
                                .stderr(File::create(stderr_file)?)
                                .current_dir(tmp_dir.path())
                                .status()?;
                            assert!(result.success());
                            println!("Finished in {:?}", time.elapsed());

                            // conversion
                            let args = format!(
                                "time target/release/intel_pt_converter --trace-path {} --output-path {}",
                                perf_data_file.display(),
                                trace_file.display(),
                            );
                            run_in_shell(&args)?;
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
                        "target/release/trace_info --trace-path {}",
                        trace_file.display(),
                    );
                    run_in_shell(&args)?;
                }
            }
        }
        Commands::SimPoint {
            tracer,
            config_name,
            parallel,
            size,
        } => {
            let tracer_name = get_tracer_name(tracer);
            let config: Config =
                serde_json::from_slice(&std::fs::read(get_config_path(config_name))?)?;

            let mut args = vec![];
            for benchmark in &config.benchmarks {
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    args.push((
                        config_name.clone(),
                        tracer_name.clone(),
                        benchmark.clone(),
                        command_index,
                        *size,
                    ));
                }
            }

            run_in_parallel(
                &args,
                *parallel,
                |(config_name, tracer_name, benchmark, command_index, size)| {
                    // trace file at "{trace_dir}/{benchmark.name}-{command_index}.log"
                    let dir = get_trace_dir(&config_name, &tracer_name);
                    std::fs::create_dir_all(&dir)?;

                    let trace_file = dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    println!("Running SimPoint on {}", trace_file.display());

                    // output prefix: "{simpoint_dir}/{benchmark.name}-{command_index}"
                    let dir = get_simpoint_dir(config_name);
                    std::fs::create_dir_all(&dir)?;
                    let output_prefix = dir.join(format!("{}-{}", benchmark.name, command_index));

                    let args = format!(
                        "target/release/simpoint --trace-path {} --size {} --output-prefix {}",
                        trace_file.display(),
                        size,
                        output_prefix.display()
                    );
                    run_in_shell(&args)?;
                    return Ok(());
                },
            )?;
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

            let per_simpoint_dir = simulate_dir.join("per-simpoint");
            create_dir_all(&per_simpoint_dir)?;

            let per_command_dir = simulate_dir.join("per-command");
            create_dir_all(&per_command_dir)?;

            let per_benchmark_dir = simulate_dir.join("per-benchmark");
            create_dir_all(&per_benchmark_dir)?;

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

                        // simulation result at "{simulate_dir}/per-simpoint/{benchmark.name}-{command_index}-simpoint-{simpoint_index}.log"
                        let output_file = per_simpoint_dir.join(format!(
                            "{}-{}-simpoint-{}.log",
                            benchmark.name, command_index, simpoint_index
                        ));
                        let args = format!(
                            "target/release/simulate --trace-path {} --predictor {} --skip 0 --warmup {} --simulate {} --output-path {}",
                            trace_file.display(),
                            predictor,
                            // half for warmup, half for simulate
                            simpoint_config.size / 2,
                            simpoint_config.size / 2,
                            output_file.display()
                        );
                        run_in_shell(&args)?;
                    }

                    // combine results of simulations
                    // combined result at "{simulate_dir}/per-command/{benchmark.name}-{command_index}.log"
                    let output_file =
                        per_command_dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    let args = format!(
                        "target/release/combine --output-path {} simpoint --simpoint-path {} --result-path {}",
                        output_file.display(),
                        get_simpoint_dir(config_name)
                            .join(format!("{}-{}.json", benchmark.name, command_index))
                            .display(),
                        per_simpoint_dir.display(),
                    );
                    run_in_shell(&args)?;
                }

                // combine results of different commands
                // combined result at "{simulate_dir}/per-benchmark/{benchmark.name}.log"
                let output_file = per_benchmark_dir.join(format!("{}.log", benchmark.name));
                let mut paths = vec![];
                for (command_index, _command) in benchmark.commands.iter().enumerate() {
                    // combined simpoint result at "{simulate_dir}/per-command/{benchmark.name}-{command_index}.log"
                    let command_file =
                        per_command_dir.join(format!("{}-{}.log", benchmark.name, command_index));
                    paths.push("--command-paths".to_string());
                    paths.push(format!("{}", command_file.display()));
                }
                let args = format!(
                    "target/release/combine --output-path {} command {}",
                    output_file.display(),
                    paths.join(" ")
                );
                run_in_shell(&args)?;
            }
        }
    }

    Ok(())
}
