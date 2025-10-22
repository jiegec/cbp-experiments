use crate::BranchType;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Read, Seek, Write},
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
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RawImage {
    pub start: u64,
    pub len: u64,
    pub data_size: u64,
    pub data_offset: u64,
    pub filename: [u8; 256],
}

impl RawImage {
    pub fn get_filename(&self) -> anyhow::Result<String> {
        let len = self
            .filename
            .iter()
            .position(|ch| *ch == 0)
            .unwrap_or(self.filename.len());
        Ok(String::from_utf8(Vec::from(&self.filename[0..len]))?)
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Entry(pub u32);

impl Entry {
    pub fn get_br_index(&self) -> usize {
        (self.0 & 0x7FFFFFFF) as usize
    }

    pub fn get_taken(&self) -> bool {
        (self.0 & 0x80000000) != 0
    }

    pub fn from(br_index: usize, taken: bool) -> Self {
        // must not overflow to taken bit
        assert!(br_index < 0x80000000);
        Self(br_index as u32 | ((taken as u32) << 31))
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
                assert!(size % std::mem::size_of::<Entry>() == 0);
                if size == 0 {
                    None
                } else {
                    let entries: &[Entry] = unsafe {
                        std::slice::from_raw_parts(
                            &self.buf[0] as *const u8 as *const Entry,
                            size / std::mem::size_of::<Entry>(),
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
        let compressed_entries = file.compressed_entries;
        let cursor = Cursor::new(compressed_entries);
        let decoder = zstd::stream::read::Decoder::new(cursor)?;
        Ok(TraceEntryIterator {
            compressed_entries: file.compressed_entries,
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
    pub num_entries: usize,
    pub num_branches: usize,
    pub num_images: usize,
    pub compressed_entries: &'a [u8],
    pub branches: &'a [Branch],
    pub images: &'a [RawImage],
}

impl<'a> TraceFileDecoder<'a> {
    pub fn open(content: &'a [u8]) -> TraceFileDecoder<'a> {
        let mut tmp_u64 = [0u8; 8];

        // read header
        tmp_u64.copy_from_slice(&content[0..8]);
        let magic = u64::from_le_bytes(tmp_u64) as usize;
        assert_eq!(magic, 0x2121505845504243);

        tmp_u64.copy_from_slice(&content[8..16]);
        let version = u64::from_le_bytes(tmp_u64) as usize;
        assert_eq!(version, 0);

        tmp_u64.copy_from_slice(&content[16..24]);
        let num_entries = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[24..32]);
        let entries_offset = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[32..40]);
        let entries_size = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[40..48]);
        let num_branches = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[48..56]);
        let branches_offset = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[56..64]);
        let num_images = u64::from_le_bytes(tmp_u64) as usize;

        tmp_u64.copy_from_slice(&content[64..72]);
        let images_offset = u64::from_le_bytes(tmp_u64) as usize;

        let images: &[RawImage] = unsafe {
            std::slice::from_raw_parts(
                &content[images_offset] as *const u8 as *const RawImage,
                num_images,
            )
        };

        let branches: &[Branch] = unsafe {
            std::slice::from_raw_parts(
                &content[branches_offset] as *const u8 as *const Branch,
                num_branches,
            )
        };

        let entries: &[u8] = unsafe {
            std::slice::from_raw_parts(&content[entries_offset] as *const u8, entries_size)
        };

        Self {
            content,
            num_entries,
            num_branches,
            num_images,
            branches,
            images,
            compressed_entries: entries,
        }
    }

    pub fn entries(&self) -> anyhow::Result<TraceEntryIterator<'a>> {
        TraceEntryIterator::from(self)
    }

    pub fn get_image_data(&self, image: &RawImage) -> &[u8] {
        &self.content[image.data_offset as usize..(image.data_offset + image.data_size) as usize]
    }

    pub fn get_images(&self) -> anyhow::Result<Vec<Image>> {
        let mut res = vec![];
        for image in self.images {
            res.push(Image::from(image, self)?);
        }
        Ok(res)
    }

    // find corresponding image & offset
    // NOTE: this reports the file offset, instead of virtual address
    // for statically linked executables, it differs by 0x400000
    pub fn get_addr_location(&self, addr: u64) -> anyhow::Result<String> {
        for image in self.images {
            if addr >= image.start && addr < image.start + image.len {
                return Ok(format!(
                    "{}:0x{:x}",
                    image.get_filename()?,
                    addr - image.start
                ));
            }
        }
        Ok(format!("unknown:0x{:x}", addr))
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
    pub images: Vec<Image>,

    // maintain mapping from (inst_addr, targ_addr) to branch index
    pub mapping: HashMap<(u64, u64), usize>,

    // output buffer
    pub buffer: [Entry; BUFFER_SIZE],
    pub buffer_size: usize,
}

impl<'a> TraceFileEncoder<'a> {
    pub fn open(file: &'a File) -> anyhow::Result<Self> {
        let mut writer = BufWriter::new(file);
        // leave space for file_header, which is 72 bytes in size
        writer.seek(std::io::SeekFrom::Start(72))?;
        Ok(Self {
            file,
            encoder: Encoder::new(writer, 0)?,
            num_entries: 0,
            branches: vec![],
            mapping: HashMap::new(),
            buffer: [Entry::default(); BUFFER_SIZE],
            buffer_size: 0,
            images: vec![],
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
                    std::mem::size_of::<Entry>() * self.buffer_size,
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
                    std::mem::size_of::<Entry>() * self.buffer_size,
                )
            })?;
            self.buffer_size = 0;
        }

        let mut writer = self.encoder.finish()?;
        let entries_offset = 72; // sizeof(file_header)
        let entries_size = writer.stream_position()? - entries_offset;

        // write branches
        let branches_offset = writer.stream_position()?;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                self.branches.as_ptr() as *const u8,
                self.branches.len() * std::mem::size_of::<Branch>(),
            )
        })?;

        // write image content
        let mut raw_images = vec![];
        for image in &self.images {
            let data_offset = writer.stream_position()?;
            writer.write_all(&image.data)?;
            let mut filename = [0u8; 256];
            let filename_bytes = image.filename.as_bytes();
            filename[0..filename_bytes.len()].copy_from_slice(filename_bytes);
            raw_images.push(RawImage {
                start: image.start,
                len: image.len,
                data_size: image.data.len() as u64,
                data_offset,
                filename,
            });
        }

        let images_offset = writer.stream_position()?;

        // write images
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                raw_images.as_ptr() as *const u8,
                raw_images.len() * std::mem::size_of::<RawImage>(),
            )
        })?;

        // write header
        writer.seek(std::io::SeekFrom::Start(0))?;
        // magic
        let val_u64 = 0x2121505845504243_u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // version
        let val_u64 = 0_u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // num_entries
        let val_u64 = self.num_entries as u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // entries_offset
        let val_u64 = entries_offset;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // entries_size
        let val_u64 = entries_size;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // num_branches
        let val_u64 = self.branches.len() as u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // branches_offset
        let val_u64 = branches_offset;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // num_images
        let val_u64 = self.images.len() as u64;
        writer.write_all(unsafe {
            std::slice::from_raw_parts(
                &val_u64 as *const u64 as *const u8,
                std::mem::size_of::<u64>(),
            )
        })?;

        // images_offset
        let val_u64 = images_offset;
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub start: u64,
    pub len: u64,
    pub data: Vec<u8>,
    pub filename: String,
}

impl Image {
    pub fn from(raw: &RawImage, decoder: &TraceFileDecoder) -> anyhow::Result<Image> {
        Ok(Image {
            start: raw.start,
            len: raw.len,
            data: Vec::from(decoder.get_image_data(raw)),
            filename: raw.get_filename()?,
        })
    }
}
