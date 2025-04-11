use cbp_experiments::get_tqdm_style;
/// Parse Intel PT trace in perf.data
use clap::Parser;
use memmap::MmapOptions;
use std::{fs::File, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to trace file
    trace_path: PathBuf,

    /// Path to executable file
    exe_path: Option<PathBuf>,
}

// for IP Compression
fn compute_ip_bytes(byte: u8) -> usize {
    let ip_bytes = byte >> 5;
    match ip_bytes {
        0x0 => 0,
        0x1 => 2,
        0x2 => 4,
        0x3 | 0x4 => 6,
        0x6 => 8,
        _ => unreachable!("Reserved IPBytes: {}", ip_bytes),
    }
}

fn compute_ip(data: &[u8], last_ip: u64) -> Option<u64> {
    let ip_bytes = compute_ip_bytes(data[0]);
    let mut target_ip = 0;
    for i in 0..ip_bytes {
        target_ip |= (data[i + 1] as u64) << (i * 8);
    }
    let mut result = 0;

    // combine
    match data[0] >> 5 {
        0x0 => None,
        0x1 => Some((target_ip & 0x000000000000ffff) | (last_ip & 0xffffffffffff0000)),
        0x2 => Some((target_ip & 0x00000000ffffffff) | (last_ip & 0xffffffff00000000)),
        0x3 => Some(
            (target_ip & 0x0000ffffffffffff)
                | (if (target_ip & 0x0000800000000000) != 0 {
                    0xffff000000000000
                } else {
                    0x0000000000000000
                }),
        ),
        0x4 => Some((target_ip & 0x0000ffffffffffff) | (last_ip & 0xffff000000000000)),
        0x6 => Some(target_ip),
        _ => unreachable!("Reserved IPBytes: {}", ip_bytes),
    }
}

fn parse_intel_pt_packets(data: &[u8]) -> anyhow::Result<()> {
    let mut offset = 0;
    let mut last_ip = 0;
    while offset < data.len() {
        match data[offset] {
            0x00 => {
                // PAD packet
                offset += 1;
            }
            0x02 if data[offset + 1] == 0x03 => {
                // CBR packet
                offset += 4;
            }
            0x02 if data[offset + 1] == 0x23 => {
                // PSBEND packet
                offset += 2;
            }
            0x02 if data[offset + 1] == 0x73 => {
                // TMA packet
                offset += 7;
            }
            0x02 if data[offset + 1] == 0x82 => {
                // PSB packet
                last_ip = 0;
                offset += 2;
            }
            0x02 if data[offset + 1] == 0xa3 => {
                // Long TNT packet
                offset += 8;
            }
            0x02 if data[offset + 1] == 0xc8 => {
                // VMCS packet
                offset += 7;
            }
            0x19 => {
                // TSC packet
                offset += 8;
            }
            0x59 => {
                // MTC packet
                offset += 2;
            }
            0x99 => {
                // MODE.Exec packet
                offset += 2;
            }
            byte @ _ if byte & 0x01 == 0x00 && byte != 0x02 => {
                // Short TNT packet
                offset += 1;
            }
            byte @ _
                if byte & 0x1f == 0x01
                    || byte & 0x1f == 0x0d
                    || byte & 0x1f == 0x11
                    || byte & 0x1f == 0x1d =>
            {
                // TIP.PGD(0x01)/TIP(0x0d)/TIP.PGE(0x11)/FUP packet
                if let Some(ip) = compute_ip(&data[offset..], last_ip) {
                    println!("Got IP 0x{:x}", ip);
                    last_ip = ip;
                }
                offset += 1 + compute_ip_bytes(byte);
            }
            byte @ _ => unimplemented!(
                "Unhandled packet byte: 0x{:x} at offset 0x{:x} with context {:x?}",
                byte,
                offset,
                &data[offset..std::cmp::min(offset + 16, data.len() - 1)]
            ),
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let file = File::open(&args.trace_path)?;
    let content = unsafe { MmapOptions::new().map(&file)? };
    // parse perf.data
    let magic = std::str::from_utf8(&content[..8])?;
    assert_eq!(magic, "PERFILE2");
    let mut tmp_u64 = [0u8; 8];
    let mut tmp_u32 = [0u8; 4];
    let mut tmp_u16 = [0u8; 2];
    // find data section offset
    tmp_u64.copy_from_slice(&content[40..48]);
    let data_section_offset = u64::from_le_bytes(tmp_u64) as usize;
    tmp_u64.copy_from_slice(&content[48..56]);
    let data_section_size = u64::from_le_bytes(tmp_u64) as usize;
    println!(
        "Found data section at 0x{:x}, size {}",
        data_section_offset, data_section_size
    );

    let pbar = indicatif::ProgressBar::new(data_section_size as u64);
    pbar.set_style(get_tqdm_style());
    let mut offset = data_section_offset;
    while offset < data_section_offset + data_section_size {
        tmp_u32.copy_from_slice(&content[offset..offset + 4]);
        let event_type = u32::from_le_bytes(tmp_u32);
        tmp_u16.copy_from_slice(&content[offset + 6..offset + 8]);
        let event_size = u16::from_le_bytes(tmp_u16);
        assert!(event_size > 0);
        // println!(
        //     "Got event at 0x{:x}: type {}, size {}",
        //     offset, event_type, event_size
        // );

        if event_type == 71 {
            // PERF_RECORD_AUXTRACE
            tmp_u64.copy_from_slice(&content[offset + 8..offset + 16]);
            let data_size = u64::from_le_bytes(tmp_u64) as usize;
            let data =
                &content[offset + event_size as usize..offset + event_size as usize + data_size];
            // println!(
            //     "Found Intel PT data at 0x{:x} with size {}",
            //     offset + event_size as usize,
            //     data_size
            // );
            parse_intel_pt_packets(data)?;
            offset += data_size;
        }
        offset += event_size as usize;

        pbar.set_position((offset - data_section_offset) as u64);
    }

    Ok(())
}
