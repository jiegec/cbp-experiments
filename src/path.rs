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
//          |- {benchmark-name}-{command-index}-stdout.log
//          \- {benchmark-name}-{command-index}.log
//       \- final/
//          \- {benchmark-name}-{command-index}.log -> ../tracer-name/{benchmark-name}-{command-index}.log

use crate::list_predictors;
use skim::{
    Skim,
    prelude::{SkimItemReader, SkimOptionsBuilder},
};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

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

fn get_selection(selections: Vec<String>, prompt: &str) -> anyhow::Result<String> {
    let options = SkimOptionsBuilder::default()
        .height(String::from("50%"))
        .prompt(prompt.to_string())
        .build()?;

    let input = selections.join("\n");

    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));

    let selected_items = Skim::run_with(&options, Some(items))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    assert_eq!(selected_items.len(), 1);
    Ok(selected_items[0].output().to_string())
}

pub fn ask_for_config_name() -> anyhow::Result<String> {
    let mut paths = vec![];
    for path in std::fs::read_dir(PathBuf::from("benchmarks"))? {
        let path = path?;
        if path.file_type()?.is_dir() {
            paths.push(format!(
                "{}",
                path.path().file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    paths.sort();

    Ok(get_selection(paths, "Choose benchmark config: ")?)
}

pub fn ask_for_simulate_dir<P: AsRef<Path>>(config_name: P) -> anyhow::Result<String> {
    let mut paths = vec![];
    for path in std::fs::read_dir(
        PathBuf::from("benchmarks")
            .join(config_name)
            .join("simulate"),
    )? {
        let path = path?;
        paths.push(format!("{}", path.path().display()));
    }
    paths.sort();
    paths.reverse();

    Ok(get_selection(paths, "Choose simulation directory: ")?)
}

pub fn ask_for_predictor() -> anyhow::Result<String> {
    let mut predictors = vec![];
    for predictor in list_predictors().iter() {
        predictors.push(predictor.to_string());
    }

    Ok(get_selection(predictors, "Choose branch predictor: ")?)
}
