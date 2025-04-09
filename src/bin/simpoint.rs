/// Test branch prediction accuracy
use cbp_experiments::{TraceFile, create_insn_index_mapping, get_tqdm_style};
use clap::Parser;
use linfa::{
    Dataset,
    traits::{Fit, Predict},
};
use linfa_clustering::KMeans;
use ndarray::{Array2, Axis, array};
use std::{path::PathBuf, ptr::null};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,

    /// Path to ELF file
    elf: PathBuf,

    /// SimPoint slice size in instructions
    size: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchInfo {
    inst_addr_index: usize,
    targ_addr_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SimPoint {
    start_instruction: u64,
    basic_block_vector: Vec<f64>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(args.trace)?;

    // parse trace file
    let file = TraceFile::open(&content);
    println!(
        "Got {} branches and {} entries",
        file.num_brs, file.num_entries
    );

    // create a mapping from instruction address to instruction index for instruction counting
    let mapping = create_insn_index_mapping(&args.elf)?;

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
    let mut simpoints: Vec<SimPoint> = vec![];
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
                simpoints.push(SimPoint {
                    start_instruction: current_simpoint_start_instruction,
                    // normalize
                    basic_block_vector: current_simpoint_basic_block_vector
                        .iter()
                        .map(|val| *val as f64 / sum_insts as f64)
                        .collect(),
                });
                current_simpoint_start_instruction = instructions;
                current_simpoint_basic_block_vector.fill(0);
            }
        }

        pbar.inc(entries.len() as u64);
    }
    pbar.finish();

    // create a new simpoint
    let sum_insts: u64 = current_simpoint_basic_block_vector.iter().sum();
    simpoints.push(SimPoint {
        start_instruction: current_simpoint_start_instruction,
        // normalize
        basic_block_vector: current_simpoint_basic_block_vector
            .iter()
            .map(|val| *val as f64 / sum_insts as f64)
            .collect(),
    });

    println!("Collected {} SimPoints", simpoints.len());

    // kmeans
    let mut vectors = Array2::<f64>::zeros((simpoints.len(), file.num_brs));
    for (i, simpoint) in simpoints.iter().enumerate() {
        for (j, val) in simpoint.basic_block_vector.iter().enumerate() {
            vectors[[i, j]] = *val;
        }
    }
    let dataset = Dataset::from(vectors.clone());
    let mut models: Vec<(KMeans<_, _>, f64)> = (2..=20)
        .map(|nclusters| {
            let model = KMeans::params(nclusters)
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
            let r = simpoints.len();
            let d = file.num_brs;
            let k = nclusters;
            let mut ri = vec![0; nclusters];
            let mut sigma = vec![0f64; nclusters];
            // find nearest cluster centroids
            let prediction = model.predict(&dataset);
            for i in 0..simpoints.len() {
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

    println!("Result: {:?}", models);

    Ok(())
}
