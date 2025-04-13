//! Use SimPoint methodology to reduce trace length
use cbp_experiments::{
    SimPointPhase, SimPointResult, TraceFileDecoder, TraceFileEncoder, create_insn_index_mapping,
    get_tqdm_style,
};
use clap::Parser;
use indicatif::ProgressIterator;
use linfa::{
    Dataset,
    traits::{Fit, Predict},
};
use linfa_clustering::KMeans;
use matplotlib::{Matplotlib, MatplotlibOpts, Mpl, Run, commands as c, serde_json::Value};
use ndarray::{Array2, Axis};
use std::{fs::File, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    #[arg(short, long)]
    trace_path: PathBuf,

    /// Path to executable file
    #[arg(short, long)]
    exe_path: PathBuf,

    /// SimPoint slice size in instructions
    #[arg(short, long)]
    size: u64,

    /// Output prefix, e.g.,
    /// json goes to: {output_prefix}.json,
    /// simpoint slices goes to: {output_prefix}-simpoint-{index}.log,
    /// plot goes to: {output_prefix}-simpoint-{index}.log
    #[arg(short, long)]
    output_prefix: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchInfo {
    inst_addr_index: u64,
    targ_addr_index: u64,
}

/// SimPoint slice: a slice is a part of the full simulation trace
#[derive(Debug, Clone, Default)]
pub struct SimPointSlice {
    /// the starting instruction
    start_instruction: u64,
    /// the ending instruction
    end_instruction: u64,
    /// basic block vector: the instructions executed in each basic block (marked by the trailing branch), normalized
    basic_block_vector: Vec<f64>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CustomPrelude;

impl Matplotlib for CustomPrelude {
    fn is_prelude(&self) -> bool {
        true
    }

    fn data(&self) -> Option<Value> {
        None
    }

    fn py_cmd(&self) -> String {
        "\
import datetime
import io
import json
import os
import random
import sys
import matplotlib
matplotlib.use(\"Agg\")
import matplotlib.path as mpath
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import matplotlib.cm as mcm
import matplotlib.colors as mcolors
import matplotlib.collections as mcollections
import matplotlib.ticker as mticker
import matplotlib.image as mimage
from mpl_toolkits.mplot3d import axes3d
import numpy as np
"
        .into()
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(&args.trace_path)?;

    // parse trace file
    let file = TraceFileDecoder::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_brs, file.num_entries
    );

    // create a mapping from instruction address to instruction index for instruction counting
    let mapping = create_insn_index_mapping(&args.exe_path)?;

    let mut branch_infos = vec![BranchInfo::default(); file.num_brs];

    // preprocess instruction indices for all branches
    for (i, branch) in file.branches.iter().enumerate() {
        branch_infos[i].inst_addr_index = *mapping.get(&branch.inst_addr).unwrap();
        branch_infos[i].targ_addr_index = *mapping.get(&branch.targ_addr).unwrap();
    }

    let pbar = indicatif::ProgressBar::new(file.num_entries as u64);
    pbar.set_style(get_tqdm_style());

    let mut last_targ_addr_index = None;
    let mut instructions = 0;
    let mut slices: Vec<SimPointSlice> = vec![];
    let mut current_simpoint_start_instruction = 0;
    let mut current_simpoint_basic_block_vector = vec![0u64; file.num_brs];
    for entries in file.entries()? {
        for entry in entries {
            let br_index = entry.get_br_index();
            let taken = entry.get_taken();

            // add instruction counting
            if taken {
                let curr_index = branch_infos[br_index].inst_addr_index;
                if let Some(last_index) = last_targ_addr_index {
                    // count instructions from last target address to the current branch address
                    assert!(curr_index >= last_index);
                    let new_insts = (curr_index - last_index + 1) as u64;
                    instructions += new_insts;

                    // sum up instructions in the basic block, marked by the ending branch
                    current_simpoint_basic_block_vector[br_index] += new_insts;
                }
                last_targ_addr_index = Some(branch_infos[br_index].targ_addr_index);
            }

            if instructions >= args.size + current_simpoint_start_instruction {
                // create a new simpoint
                let sum_insts: u64 = current_simpoint_basic_block_vector.iter().sum();
                slices.push(SimPointSlice {
                    start_instruction: current_simpoint_start_instruction,
                    end_instruction: args.size + current_simpoint_start_instruction,
                    // normalize
                    basic_block_vector: current_simpoint_basic_block_vector
                        .iter()
                        .map(|val| *val as f64 / sum_insts as f64)
                        .collect(),
                });
                current_simpoint_start_instruction += args.size;
                current_simpoint_basic_block_vector.fill(0);
            }
        }

        pbar.inc(entries.len() as u64);
    }
    pbar.finish();

    let total_instructions = instructions;

    // create a new simpoint
    let sum_insts: u64 = current_simpoint_basic_block_vector.iter().sum();
    slices.push(SimPointSlice {
        start_instruction: current_simpoint_start_instruction,
        end_instruction: args.size + current_simpoint_start_instruction,
        // normalize
        basic_block_vector: current_simpoint_basic_block_vector
            .iter()
            .map(|val| *val as f64 / sum_insts as f64)
            .collect(),
    });

    println!(
        "Collected {} SimPoint slices, running K-Means",
        slices.len()
    );

    // kmeans
    let mut vectors = Array2::<f64>::zeros((slices.len(), file.num_brs));
    for (i, simpoint) in slices.iter().enumerate() {
        for (j, val) in simpoint.basic_block_vector.iter().enumerate() {
            vectors[[i, j]] = *val;
        }
    }
    let dataset = Dataset::from(vectors.clone());
    let mut models: Vec<(KMeans<_, _>, f64)> = (1..21)
        .progress()
        .map(|num_clusters| {
            let model = KMeans::params(num_clusters)
                .tolerance(1e-2)
                .fit(&dataset)
                .unwrap();

            // compute BIC(Bayesian Information Criterion)
            // parameters:
            // R: the number of points in the data i.e. simpoints.len()
            // d: the dimension of basic block vectors i.e. file.num_brs
            // k: the number of clusters i.e. nclusters
            // Ri: the number of points in the i-th cluster
            // sigma^2: the average variance from each point to its cluster center of the i-th cluster
            // BIC = sum(-Ri*log(2*pi)/2-Ri*d*log(sigma^2)/2-(Ri-1)/2+Ri*log(Ri/R))-(k+d*k)*log(R)/2
            let r = slices.len();
            let d = file.num_brs;
            let k = num_clusters;
            let mut ri = vec![0; num_clusters];
            let mut sigma = vec![0f64; num_clusters];
            // find nearest cluster centroids
            let prediction = model.predict(&dataset);
            for i in 0..slices.len() {
                let cluster = prediction[i];
                ri[cluster] += 1;
                let closest_centroid = &model.centroids().index_axis(Axis(0), cluster);
                for j in 0..file.num_brs {
                    let diff = vectors[[i, j]] - closest_centroid[j];
                    sigma[cluster] += diff * diff;
                }
            }
            // normalize sigma
            for i in 0..k {
                sigma[i] /= ri[i] as f64;
                // avoid zero sigma
                sigma[i] += 1e-6;
            }
            // compute BIC
            let mut bic = 0.0;
            // sum(-Ri*log(2*pi)/2-Ri*d*log(sigma^2)/2-(Ri-1)/2+Ri*log(Ri/R))
            for i in 0..k {
                bic -= ri[i] as f64 * f64::ln(2.0 * std::f64::consts::PI) / 2.0;
                bic -= ri[i] as f64 * d as f64 * f64::ln(sigma[i]) / 2.0;
                bic -= (ri[i] - 1) as f64 / 2.0;
                bic += ri[i] as f64 * f64::ln(ri[i] as f64 / r as f64);
            }
            // -(k+d*k)*log(R)/2
            bic -= (k + d * k) as f64 * f64::ln(r as f64) / 2.0;
            (model, bic)
        })
        .collect();

    models.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap());

    // find the first model that: larger than 90% of the spread between the largest and smallest BIC
    let smallest_score = models[0].1;
    let largest_score = models[models.len() - 1].1;
    let threshold = smallest_score * 0.1 + largest_score * 0.9;

    let best_model = &models.iter().find(|model| model.1 >= threshold).unwrap().0;

    // find nearest cluster centroids
    let prediction = best_model.predict(&dataset);
    let num_clusters = best_model.cluster_count().dim();
    println!("Got {} clusters", num_clusters);

    // for each cluster (phase):
    // 1. count points that belong to it
    // 2. find the nearest point to it
    let mut phase_weights = vec![0; num_clusters];
    let mut phase_nearest = vec![None; num_clusters];

    for i in 0..slices.len() {
        let cluster = prediction[i];
        phase_weights[cluster] += 1;
        // compute distance
        let mut dist = 0.0;
        for j in 0..file.num_brs {
            let diff = vectors[[i, j]] - best_model.centroids()[[cluster, j]];
            dist += diff * diff;
        }
        match phase_nearest[cluster] {
            None => {
                phase_nearest[cluster] = Some((i, dist));
            }
            Some((_, old_dist)) if old_dist > dist => {
                phase_nearest[cluster] = Some((i, dist));
            }
            _ => {}
        }
    }

    // save simpoint phases
    let mut phases = vec![];
    for i in 0..num_clusters {
        phases.push(SimPointPhase {
            weight: phase_weights[i],
            start_instruction: slices[phase_nearest[i].unwrap().0].start_instruction,
            end_instruction: slices[phase_nearest[i].unwrap().0].end_instruction,
        });
    }
    // sort by start instruction
    phases.sort_by_key(|phase| phase.start_instruction);

    // iterate entries again and save the representative slice in each phase
    println!("Saving {} slices", phases.len());
    let mut trace_files = vec![];
    let mut encoders = vec![];
    println!(
        "Creating SimPoint slices at {}-simpoint-[{}-{}].log",
        args.output_prefix,
        0,
        phases.len() - 1
    );
    for (phase_index, _phase) in phases.iter().enumerate() {
        let trace_path = format!("{}-simpoint-{}.log", args.output_prefix, phase_index);
        trace_files.push(File::create(&trace_path)?);
    }
    for trace_file in &trace_files {
        let mut encoder = TraceFileEncoder::open(trace_file)?;
        // for simplicity, copy all branches instead of re-creating one on the fly
        encoder.branches = file.branches.to_vec();
        encoders.push(encoder);
    }

    let pbar = indicatif::ProgressBar::new(file.num_entries as u64);
    pbar.set_style(get_tqdm_style());

    let mut last_targ_addr_index = None;
    let mut instructions = 0;
    let mut current_phase_index = 0;
    for entries in file.entries()? {
        for entry in entries {
            let br_index = entry.get_br_index();
            let taken = entry.get_taken();

            // add instruction counting
            if taken {
                let curr_index = branch_infos[br_index].inst_addr_index;
                if let Some(last_index) = last_targ_addr_index {
                    // count instructions from last target address to the current branch address
                    assert!(curr_index >= last_index);
                    let new_insts = (curr_index - last_index + 1) as u64;
                    instructions += new_insts;
                }
                last_targ_addr_index = Some(branch_infos[br_index].targ_addr_index);
            }

            // beyond the current simpoint representative slice?
            if instructions > phases[current_phase_index].end_instruction {
                current_phase_index += 1;
            }

            // all slices are finished?
            if current_phase_index == phases.len() {
                break;
            }

            // within the current simpoint representative slice?
            if instructions >= phases[current_phase_index].start_instruction
                && instructions <= phases[current_phase_index].end_instruction
            {
                encoders[current_phase_index].record_event_with_branch_index(br_index, taken)?;
            }
        }

        // all slices are finished?
        if current_phase_index == phases.len() {
            break;
        }

        pbar.inc(entries.len() as u64);
    }
    pbar.finish();

    // finish each slice
    for encoder in encoders {
        encoder.finish()?;
    }

    let result = SimPointResult {
        trace_path: args.trace_path.clone(),
        exe_path: args.exe_path.clone(),
        size: args.size,
        total_instructions,
        phases,
    };

    let json_path = format!("{}.json", args.output_prefix);
    std::fs::write(&json_path, serde_json::to_vec_pretty(&result)?)?;
    println!("SimPoint configuration written to {}", json_path);

    // plot
    let plot_path = format!("{}.png", args.output_prefix);
    Mpl::new()
        & CustomPrelude
        & c::DefInit
        & c::plot(
            (0..slices.len()).map(|num| num as f64),
            prediction.map(|num| *num as f64),
        )
        .o("marker", "s")
        .o("linestyle", "")
        & c::yticks((0..num_clusters).map(|num| num as f64))
        | Run::Save(PathBuf::from(&plot_path));
    println!("Visualization generated to {}", plot_path);

    Ok(())
}
