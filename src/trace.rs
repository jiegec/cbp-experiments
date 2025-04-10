use std::io::{BufReader, Cursor, Read};
use zstd::stream::read::Decoder;

// follow definitions in common.h

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Branch {
    pub inst_addr: u64,
    pub targ_addr: u64,
    pub inst_length: u32,
    pub branch_type: BranchType,
}

#[repr(C)]
pub struct Entry(pub u16);

impl Entry {
    pub fn get_br_index(&self) -> usize {
        (self.0 & 0x7FFF).into()
    }

    pub fn get_taken(&self) -> bool {
        (self.0 & 0x8000) != 0
    }
}

pub type BranchType = crate::ffi::BranchType;

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
    pub fn from(file: &TraceFile<'a>) -> anyhow::Result<TraceEntryIterator<'a>> {
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

pub struct TraceFile<'a> {
    // raw trace file content
    pub content: &'a [u8],

    // parse content
    pub num_brs: usize,
    pub num_entries: usize,
    pub branches: &'a [Branch],
}

impl<'a> TraceFile<'a> {
    pub fn open(content: &'a [u8]) -> TraceFile<'a> {
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
