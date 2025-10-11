use crate::BranchType;
use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, path::Path};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TagePHRXorConfig {
    /// B(i): B[i] is xorred
    B(usize),
    /// T(i): T[i] is xorred
    T(usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagePHRConfig {
    /// PHR register name
    name: String,
    /// PHR length in bits
    length: usize,
    /// PHR shift amount per taken branch
    shift: usize,
    /// Computation formula of PHR footprints, from MSB to LSB
    /// each bit of footprint is xored from one or more bits
    footprint: Vec<Vec<TagePHRXorConfig>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TageHistoryRegisterConfig {
    /// Path history register
    PHR(TagePHRConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TageXorConfig {
    /// HR(i, j): HR[i][j] is xorred, i is the index of history register, j is the bit index
    HR(usize, usize),
    /// PC(i): PC[i] is xorred
    PC(usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TageTableConfig {
    /// Computation formula of index bits, from MSB to LSB
    /// each bit of index is xored from one or more bits
    index_bits: Vec<Vec<TageXorConfig>>,

    /// Computation formula of tag bits, from MSB to LSB
    /// each bit of tag is xored from one or more bits
    tag_bits: Vec<Vec<TageXorConfig>>,

    /// Set associative
    ways: usize,

    /// Width of counter in each entry
    counter_width: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TageConfig {
    /// One or more more history registers
    history_registers: Vec<TageHistoryRegisterConfig>,
    /// One or more pattern history tables
    tables: Vec<TageTableConfig>,
}

#[derive(Clone, Debug)]
pub struct TageHistoryRegister {
    bits: BitVec,
    config: TageHistoryRegisterConfig,
}

impl TageHistoryRegister {
    pub fn update(&mut self, branch_addr: u64, target_addr: u64) {
        match &self.config {
            TageHistoryRegisterConfig::PHR(tage_phrconfig) => {
                // step 1: shift
                // the bitvec is lsb first
                self.bits.shift_right(tage_phrconfig.shift);

                // step 2: xor footprint
                for (bit, formula) in tage_phrconfig.footprint.iter().rev().enumerate() {
                    let mut computed = 0;
                    for entry in formula {
                        let b = match entry {
                            TagePHRXorConfig::B(i) => (branch_addr >> *i) & 1,
                            TagePHRXorConfig::T(i) => (target_addr >> *i) & 1,
                        };
                        computed ^= b;
                    }
                    let new_value = self.bits.get(bit).unwrap().deref()
                        ^ match computed {
                            0 => false,
                            1 => true,
                            _ => unreachable!(),
                        };
                    self.bits.set(bit, new_value);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TageTableEntry {
    tag: u16,
    counter: u8,
    useful: u8,
}

#[derive(Clone, Debug)]
pub struct TageTable {
    /// (2 ** index_bits.len()) * ways
    entries: Vec<TageTableEntry>,
    config: TageTableConfig,
}

impl TageTable {
    pub fn compute(
        &self,
        pc: u64,
        history_registers: &[TageHistoryRegister],
        bits: &Vec<Vec<TageXorConfig>>,
    ) -> usize {
        let mut index = 0;
        for (bit, formula) in bits.iter().enumerate() {
            let mut computed = 0;
            for entry in formula {
                let b = match entry {
                    TageXorConfig::HR(i, j) => {
                        *history_registers[*i].bits.get(*j).unwrap().deref() as u64
                    }
                    TageXorConfig::PC(i) => (pc >> *i) & 1,
                };
                computed ^= b;
            }
            index += computed << bit;
        }
        index as usize
    }

    pub fn get_index(&self, pc: u64, history_registers: &[TageHistoryRegister]) -> usize {
        self.compute(pc, history_registers, &self.config.index_bits)
    }

    pub fn get_tag(&self, pc: u64, history_registers: &[TageHistoryRegister]) -> usize {
        self.compute(pc, history_registers, &self.config.tag_bits)
    }
}

#[derive(Clone, Debug)]
pub struct Tage {
    config: TageConfig,
    tables: Vec<TageTable>,
    history_registers: Vec<TageHistoryRegister>,
}

impl Tage {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Tage> {
        let config: TageConfig = toml::from_str(&std::fs::read_to_string(path)?)?;

        let mut tables = vec![];
        for table_config in &config.tables {
            tables.push(TageTable {
                entries: vec![
                    TageTableEntry {
                        tag: 0,
                        counter: 0,
                        useful: 0
                    };
                    (1 << table_config.index_bits.len()) * table_config.ways
                ],
                config: table_config.clone(),
            });
        }

        let mut history_registers = vec![];
        for hr_config in &config.history_registers {
            let mut bits = BitVec::new();
            match hr_config {
                TageHistoryRegisterConfig::PHR(tage_phrconfig) => {
                    bits.resize(tage_phrconfig.length, false);
                }
            }
            history_registers.push(TageHistoryRegister {
                bits,
                config: hr_config.clone(),
            });
        }

        Ok(Tage {
            config,
            tables,
            history_registers,
        })
    }

    pub fn predict(&mut self, pc: u64, groundtruth: bool) -> bool {
        true
    }

    pub fn update(
        &mut self,
        pc: u64,
        branch_type: BranchType,
        resolve_direction: bool,
        predict_direction: bool,
        branch_target: u64,
    ) {
        // TODO: update tage

        for hr in &mut self.history_registers {
            hr.update(pc, branch_target);
        }
    }

    pub fn update_others(
        &mut self,
        pc: u64,
        branch_type: BranchType,
        branch_taken: bool,
        branch_target: u64,
    ) {
        for hr in &mut self.history_registers {
            hr.update(pc, branch_target);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Tage, TageConfig, TageHistoryRegisterConfig, TagePHRConfig, TagePHRXorConfig,
        TageTableConfig, TageXorConfig,
    };

    #[test]
    fn test_firestorm() {
        let tage = Tage::new("configs/firestorm.toml").unwrap();
        println!("Parsed {:?}", tage);
        println!("History registers:");
        for (index, hr) in tage.config.history_registers.iter().enumerate() {
            match hr {
                TageHistoryRegisterConfig::PHR(config) => {
                    println!(
                        "    History register {index}({}): {} bits in length, {} bit shift per taken branch, {} bit footprint",
                        config.name,
                        config.length,
                        config.shift,
                        config.footprint.len()
                    )
                }
            }
        }

        println!("Tables:");

        for (index, config) in tage.config.tables.iter().enumerate() {
            // for each history register, find maximum bit index used
            let mut history_length = vec![];
            let mut hr_names = vec![];
            for i in 0..(tage.config.history_registers.len()) {
                let mut max: Option<usize> = None;
                for bit in config.index_bits.iter().chain(config.tag_bits.iter()) {
                    for element in bit {
                        if let TageXorConfig::HR(index, bit) = element {
                            if i == *index && (max.is_none() || max < Some(*bit + 1)) {
                                max = Some(*bit + 1);
                            }
                        }
                    }
                }
                history_length.push(max);

                match &tage.config.history_registers[i] {
                    TageHistoryRegisterConfig::PHR(config) => hr_names.push(config.name.clone()),
                }
            }
            println!(
                "    Table {index}: {} way, {} index bits, {} tag bits, history length {:?}, {} entries",
                config.ways,
                config.index_bits.len(),
                config.tag_bits.len(),
                history_length,
                (1 << config.index_bits.len()) * config.ways,
            );

            // print index bits
            for (i, bits) in config.index_bits.iter().enumerate() {
                println!(
                    "        Index bit {i}: {}",
                    bits.iter()
                        .map(|cfg| match cfg {
                            TageXorConfig::HR(index, bit) => format!("{}[{bit}]", hr_names[*index]),
                            TageXorConfig::PC(bit) => format!("PC[{bit}]"),
                        })
                        .collect::<Vec<String>>()
                        .join(" xor ")
                );
            }

            // print tag bits
            for (i, bits) in config.tag_bits.iter().enumerate() {
                println!(
                    "        Tag bit {i}: {}",
                    bits.iter()
                        .map(|cfg| match cfg {
                            TageXorConfig::HR(index, bit) => format!("{}[{bit}]", hr_names[*index]),
                            TageXorConfig::PC(bit) => format!("PC[{bit}]"),
                        })
                        .collect::<Vec<String>>()
                        .join(" xor ")
                );
            }
        }
    }

    #[test]
    fn test_serialize() {
        println!(
            "{}",
            toml::to_string(&TageConfig {
                history_registers: vec![
                    TageHistoryRegisterConfig::PHR(TagePHRConfig {
                        name: "PHR1".to_string(),
                        length: 2,
                        shift: 1,
                        footprint: vec![
                            vec![TagePHRXorConfig::B(0), TagePHRXorConfig::T(1)],
                            vec![TagePHRXorConfig::B(2), TagePHRXorConfig::T(3)]
                        ]
                    }),
                    TageHistoryRegisterConfig::PHR(TagePHRConfig {
                        name: "PHR2".to_string(),
                        length: 3,
                        shift: 4,
                        footprint: vec![
                            vec![TagePHRXorConfig::B(4), TagePHRXorConfig::T(5)],
                            vec![TagePHRXorConfig::B(6), TagePHRXorConfig::T(7)]
                        ]
                    })
                ],
                tables: vec![
                    TageTableConfig {
                        index_bits: vec![vec![TageXorConfig::PC(0), TageXorConfig::HR(1, 2)]],
                        tag_bits: vec![vec![TageXorConfig::PC(3), TageXorConfig::HR(4, 5)]],
                        ways: 4,
                        counter_width: 2,
                    },
                    TageTableConfig {
                        index_bits: vec![vec![TageXorConfig::PC(6), TageXorConfig::HR(7, 8)]],
                        tag_bits: vec![vec![TageXorConfig::PC(9), TageXorConfig::HR(10, 11)]],
                        ways: 4,
                        counter_width: 2,
                    }
                ],
            })
            .unwrap()
        );
    }
}
