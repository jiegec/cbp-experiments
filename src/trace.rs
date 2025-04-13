use crate::BranchType;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Read, Write},
};
use zstd::{Encoder, stream::read::Decoder};

// follow definitions in common.h

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Branch {
    pub inst_addr: u64,
    pub targ_addr: u64,
    pub inst_length: u32,
    pub branch_type: BranchType,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Entry(pub u16);

impl Entry {
    pub fn get_br_index(&self) -> usize {
        (self.0 & 0x7FFF).into()
    }

    pub fn get_taken(&self) -> bool {
        (self.0 & 0x8000) != 0
    }

    pub fn from(br_index: usize, taken: bool) -> Self {
        // max brs
        assert!(br_index < 0x8000);
        Self(br_index as u16 | ((taken as u16) << 15))
    }
}

pub struct TraceEntryIterator<'a> {
    pub compressed_entries: &'a [u8],
    pub num_entries: usize,
    pub buf: [u8; 1024 * 256],
    pub decoder: Decoder<'a, BufReader<Cursor<&'a [u8]>>>,
}

impl<'a> Iterator for TraceEntryIterator<'a> {
    type Item = &'a [Entry];

    fn next(&mut self) -> Option<Self::Item> {
        // ask for more data from decoder
        match self.decoder.read(&mut self.buf) {
            Ok(size) => {
                assert!(size % 2 == 0);
                if size == 0 {
                    None
                } else {
                    let entries: &[Entry] = unsafe {
                        std::slice::from_raw_parts(
                            &self.buf[0] as *const u8 as *const Entry,
                            size / 2,
                        )
                    };
                    Some(entries)
                }
            }
            Err(err) => {
                panic!(
                    "Unexpected error to read data from zstd compressed stream: {:?}",
                    err
                );
            }
        }
    }
}

impl<'a> TraceEntryIterator<'a> {
    pub fn from(file: &TraceFileDecoder<'a>) -> anyhow::Result<TraceEntryIterator<'a>> {
        let compressed_entries = &file.content
            [0..file.content.len() - 16 - std::mem::size_of::<Branch>() * file.num_brs];
        let cursor = Cursor::new(compressed_entries);
        let decoder = zstd::stream::read::Decoder::new(cursor)?;
        Ok(TraceEntryIterator {
            compressed_entries: &file.content
                [0..file.content.len() - 16 - std::mem::size_of::<Branch>() * file.num_brs],
            num_entries: file.num_entries,
            buf: [0u8; 1024 * 256],
            decoder,
        })
    }
}

pub struct TraceFileDecoder<'a> {
    // raw trace file content
    pub content: &'a [u8],

    // parse content
    pub num_brs: usize,
    pub num_entries: usize,
    pub branches: &'a [Branch],
}

impl<'a> TraceFileDecoder<'a> {
    pub fn open(content: &'a [u8]) -> TraceFileDecoder<'a> {
        // read num_brs
        let mut tmp_u64 = [0u8; 8];
        tmp_u64.copy_from_slice(&content[content.len() - 16..content.len() - 8]);
        let num_brs = u64::from_le_bytes(tmp_u64) as usize;
        tmp_u64.copy_from_slice(&content[content.len() - 8..content.len()]);
        let num_entries = u64::from_le_bytes(tmp_u64) as usize;

        let branches: &[Branch] = unsafe {
            std::slice::from_raw_parts(
                &content[content.len() - 16 - std::mem::size_of::<Branch>() * num_brs] as *const u8
                    as *const Branch,
                num_brs,
            )
        };

        Self {
            content,
            num_brs,
            num_entries,
            branches,
        }
    }

    pub fn entries(&self) -> anyhow::Result<TraceEntryIterator<'a>> {
        TraceEntryIterator::from(self)
    }
}

const BUFFER_SIZE: usize = 16384;

pub struct TraceFileEncoder<'a> {
    // trace file
    pub file: &'a File,
    pub encoder: Encoder<'a, BufWriter<&'a File>>,

    // content
    pub num_entries: usize,
    pub branches: Vec<Branch>,

    // maintain mapping from (inst_addr, targ_addr) to branch index
    pub mapping: HashMap<(u64, u64), usize>,

    // output buffer
    pub buffer: [Entry; BUFFER_SIZE],
    pub buffer_size: usize,
}

impl<'a> TraceFileEncoder<'a> {
    pub fn open(file: &'a File) -> anyhow::Result<Self> {
        Ok(Self {
            file,
            encoder: Encoder::new(BufWriter::new(file), 0)?,
            num_entries: 0,
            branches: vec![],
            mapping: HashMap::new(),
            buffer: [Entry::default(); BUFFER_SIZE],
            buffer_size: 0,
        })
    }

    /// Returns internal branch index for optimization
    pub fn record_event(
        &mut self,
        inst_addr: u64,
        targ_addr: u64,
        inst_length: u32,
        branch_type: BranchType,
        taken: bool,
    ) -> anyhow::Result<usize> {
        let br_index = match self.mapping.get(&(inst_addr, targ_addr)) {
            Some(index) => *index,
            None => {
                let index = self.branches.len();
                self.branches.push(Branch {
                    inst_addr,
                    targ_addr,
                    inst_length,
                    branch_type,
                });
                self.mapping.insert((inst_addr, targ_addr), index);
                index
            }
        };

        self.record_event_with_branch_index(br_index, taken)?;

        Ok(br_index)
    }

    /// If the caller already knows the branch index, use this
    pub fn record_event_with_branch_index(
        &mut self,
        br_index: usize,
        taken: bool,
    ) -> anyhow::Result<()> {
        let entry = Entry::from(br_index, taken);
        self.buffer[self.buffer_size] = entry;
        self.buffer_size += 1;

        if self.buffer_size == BUFFER_SIZE {
            // flush
            self.encoder.write_all(unsafe {
                std::slice::from_raw_parts(
                    self.buffer.as_ptr() as *const u8,
                    std::mem::size_of::<u16>() * self.buffer_size,
                )
            })?;
            self.buffer_size = 0;
        }

        self.num_entries += 1;
        Ok(())
    }

    pub fn finish(mut self) -> anyhow::Result<()> {
        if self.buffer_size > 0 {
            // flush
            self.encoder.write_all(unsafe {
                std::slice::from_raw_parts(
                    self.buffer.as_ptr() as *const u8,
                    std::mem::size_of::<u16>() * self.buffer_size,
                )
            })?;
            self.buffer_size = 0;
        }

        let mut writer = self.encoder.finish()?;
        // write branches
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                self.branches.as_ptr() as *const u8,
                self.branches.len() * std::mem::size_of::<Branch>(),
            )
        })?;

        // write num_brs and num_entries
        let val_u64 = self.branches.len() as u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        let val_u64 = self.num_entries as u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;
        writer.flush()?;
        Ok(())
    }
}
