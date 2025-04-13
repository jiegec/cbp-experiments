// benchmark folder structure:
// benchmarks/
// \- {config-name}/
//    |- config.json
//    |- simpoint/
//       |- {benchmark-name}-{command-index}-simpoint-{command-index}.log
//       |- {benchmark-name}-{command-index}.png
//       \- {benchmark-name}-{command-index}.json
//    |- simulate/
//       \- {datetime}-{predictor}/
//          |- per-benchmark/
//             \- {benchmark-name}.log
//          |- per-command/
//             \- {benchmark-name}-{command-index}.log
//          \- per-simpoint/
//             \- {benchmark-name}-{command-index}-simpoint-{command-index}.log
//    \- traces/
//       \- {tracer-name}/
//          \- {benchmark-name}-{command-index}.log
//       \- final/
//          \- {benchmark-name}-{command-index}.log -> ../tracer-name/{benchmark-name}-{command-index}.log

use std::path::{Path, PathBuf};

pub fn get_config_path<P: AsRef<Path>>(config_name: P) -> PathBuf {
    PathBuf::from("benchmarks")
        .join(config_name)
        .join("config.json")
}

pub fn get_simpoint_dir<P: AsRef<Path>>(config_name: P) -> PathBuf {
    PathBuf::from("benchmarks")
        .join(config_name)
        .join("simpoint")
}

pub fn get_trace_dir<P1: AsRef<Path>, P2: AsRef<Path>>(
    config_name: P1,
    tracer_name: P2,
) -> PathBuf {
    PathBuf::from("benchmarks")
        .join(config_name)
        .join("traces")
        .join(tracer_name)
}

pub fn get_simulate_dir<P: AsRef<Path>>(
    config_name: P,
    datetime: &str,
    predictor: &str,
) -> PathBuf {
    PathBuf::from("benchmarks")
        .join(config_name)
        .join("simulate")
        .join(format!("{}-{}", datetime, predictor))
}
