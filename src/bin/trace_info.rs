use cbp_experiments::{Branch, Entry};
use clap::Parser;
use core::slice;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let content = std::fs::read(args.trace)?;
    // read num_brs
    let mut tmp_u64 = [0u8; 8];
    tmp_u64.copy_from_slice(&content[content.len() - 16..content.len() - 8]);
    let num_brs = u64::from_le_bytes(tmp_u64);
    tmp_u64.copy_from_slice(&content[content.len() - 8..content.len()]);
    let num_entries = u64::from_le_bytes(tmp_u64);
    println!("Got {num_brs} branches and {num_entries} entries");

    let branches: &[Branch] = unsafe {
        slice::from_raw_parts(
            &content[content.len() - 16 - std::mem::size_of::<Branch>() * num_brs as usize]
                as *const u8 as *const Branch,
            num_brs as usize,
        )
    };

    let compressed_entries: &[u8] =
        &content[0..content.len() - 16 - std::mem::size_of::<Branch>() * num_brs as usize];
    let cursor = Cursor::new(compressed_entries);
    let mut decoder = zstd::stream::read::Decoder::new(cursor)?;

    loop {
        let mut buf = [0u8; 2];
        match decoder.read_exact(&mut buf) {
            Ok(()) => {
                let entry_raw = u16::from_le_bytes(buf);
                let entry = Entry(entry_raw);
                let branch = branches[entry.get_br_index()];
                println!(
                    "Got entry {} {:x?} {}",
                    entry.get_br_index(),
                    branch,
                    entry.get_taken()
                );
            }
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to read data from zstd compressed stream: {:?}",
                    err
                ));
            }
        }
    }

    Ok(())
}
