use crate::{BranchType, ConditionalBranchPredictor};
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
pub struct TageBaseTableConfig {
    /// Computation formula of index bits, from MSB to LSB
    /// each bit of index is xored from one or more bits
    index_bits: Vec<Vec<TageXorConfig>>,

    /// Width of counter in each entry
    counter_width: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TageConfig {
    /// One or more more history registers
    history_registers: Vec<TageHistoryRegisterConfig>,
    /// Base table
    base_table: TageBaseTableConfig,
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

impl TageTableEntry {
    pub fn get_prediction(&self, counter_width: usize) -> bool {
        let taken_limit = 1 << (counter_width - 1);
        self.counter >= taken_limit
    }

    pub fn increment_counter(&mut self, counter_width: usize) {
        let limit = (1 << counter_width) - 1;
        self.counter = if self.counter == limit {
            limit
        } else {
            self.counter + 1
        };
    }

    pub fn decrement_counter(&mut self) {
        self.counter = if self.counter == 0 {
            0
        } else {
            self.counter - 1
        };
    }

    pub fn increment_useful(&mut self) {
        // 2-bit
        self.useful = if self.useful == 3 { 3 } else { self.useful + 1 };
    }

    pub fn decrement_useful(&mut self) {
        // 2-bit
        self.useful = if self.useful == 0 { 0 } else { self.useful - 1 };
    }
}

#[derive(Clone, Debug)]
pub struct TageTable {
    /// (2 ** index_bits.len()) * ways
    entries: Vec<TageTableEntry>,
    config: TageTableConfig,
}

impl TageTable {
    pub fn find_match(&self, pc: u64, history_registers: &[TageHistoryRegister]) -> Option<usize> {
        let index = self.get_index(pc, history_registers);
        let tag = self.get_tag(pc, history_registers);
        for i in 0..self.config.ways {
            // find match
            let j = index * self.config.ways + i;
            if self.entries[j].tag as usize == tag {
                // found
                return Some(j);
            }
        }
        None
    }

    fn compute(
        pc: u64,
        history_registers: &[TageHistoryRegister],
        bits: &[Vec<TageXorConfig>],
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
        Self::compute(pc, history_registers, &self.config.index_bits)
    }

    pub fn get_tag(&self, pc: u64, history_registers: &[TageHistoryRegister]) -> usize {
        Self::compute(pc, history_registers, &self.config.tag_bits)
    }

    pub fn allocate(
        &mut self,
        pc: u64,
        history_registers: &[TageHistoryRegister],
        direction: bool,
        counter_width: usize,
    ) -> bool {
        let index = self.get_index(pc, history_registers);
        for i in 0..self.config.ways {
            // find zero useful
            let j = index * self.config.ways + i;
            if self.entries[j].useful == 0 {
                // allocate
                let tag = self.get_tag(pc, history_registers);
                self.entries[j].tag = tag as u16;
                self.entries[j].useful = 0;
                if direction {
                    // weak taken
                    self.entries[j].counter = 1 << (counter_width - 1);
                } else {
                    // weak not taken
                    self.entries[j].counter = (1 << (counter_width - 1)) - 1;
                }
                return true;
            }
        }
        false
    }

    pub fn decrement_useful(&mut self, pc: u64, history_registers: &[TageHistoryRegister]) -> bool {
        let index = self.get_index(pc, history_registers);
        for i in 0..self.config.ways {
            let j = index * self.config.ways + i;
            self.entries[j].decrement_useful();
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct TageBaseTableEntry {
    counter: u8,
}

impl TageBaseTableEntry {
    pub fn get_prediction(&self, counter_width: usize) -> bool {
        let taken_limit = 1 << (counter_width - 1);
        self.counter >= taken_limit
    }

    pub fn increment_counter(&mut self, counter_width: usize) {
        let limit = (1 << counter_width) - 1;
        self.counter = if self.counter == limit {
            limit
        } else {
            self.counter + 1
        };
    }

    pub fn decrement_counter(&mut self) {
        self.counter = if self.counter == 0 {
            0
        } else {
            self.counter - 1
        };
    }
}

#[derive(Clone, Debug)]
pub struct TageBaseTable {
    /// 2 ** index_bits.len()
    entries: Vec<TageBaseTableEntry>,
    config: TageBaseTableConfig,
}

impl TageBaseTable {
    pub fn get_index(&self, pc: u64, history_registers: &[TageHistoryRegister]) -> usize {
        TageTable::compute(pc, history_registers, &self.config.index_bits)
    }
}

#[derive(Clone, Debug)]
pub struct TageMatchFromBase {
    entry_index: usize,
}

#[derive(Clone, Debug)]
pub struct TageMatchFromNonBase {
    table: usize,
    entry_index: usize,
}

#[derive(Clone, Debug)]
pub enum TageMatchInner {
    Base(TageMatchFromBase),
    NonBase(TageMatchFromNonBase),
}

#[derive(Clone, Debug)]
pub struct TageMatch {
    pred: Option<TageMatchInner>,
    altpred: Option<TageMatchInner>,
}

#[derive(Clone, Debug)]
pub struct Tage {
    config: TageConfig,
    base_table: TageBaseTable,
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

        let base_table = TageBaseTable {
            entries: vec![
                TageBaseTableEntry { counter: 0 };
                1 << config.base_table.index_bits.len()
            ],
            config: config.base_table.clone(),
        };

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
            base_table,
            history_registers,
        })
    }

    fn find_match(&self, pc: u64) -> TageMatch {
        let mut res = TageMatch {
            pred: None,
            altpred: None,
        };

        // generate prediction from base table
        let entry_index = self.base_table.get_index(pc, &self.history_registers);
        res.pred = Some(TageMatchInner::Base(TageMatchFromBase { entry_index }));

        for i in 0..self.config.tables.len() {
            if let Some(entry_index) = self.tables[i].find_match(pc, &self.history_registers) {
                res.altpred = res.pred;
                res.pred = Some(TageMatchInner::NonBase(TageMatchFromNonBase {
                    table: i,
                    entry_index,
                }));
            }
        }
        res
    }
}

impl ConditionalBranchPredictor for Tage {
    fn predict(&mut self, pc: u64, _groundtruth: bool) -> bool {
        let m = self.find_match(pc);
        match m.pred.unwrap() {
            TageMatchInner::Base(pred) => {
                let entry = &self.base_table.entries[pred.entry_index];
                entry.get_prediction(self.config.base_table.counter_width)
            }
            TageMatchInner::NonBase(pred) => {
                let entry = &self.tables[pred.table].entries[pred.entry_index];
                entry.get_prediction(self.config.tables[pred.table].counter_width)
            }
        }
    }

    fn update(
        &mut self,
        pc: u64,
        branch_type: BranchType,
        resolve_direction: bool,
        predict_direction: bool,
        branch_target: u64,
    ) {
        // update tage
        if let BranchType::ConditionalDirectJump = branch_type {
            let m = self.find_match(pc);
            let mut min_table = 0;
            if let TageMatchInner::NonBase(pred) = m.pred.unwrap() {
                min_table = pred.table + 1;
                let pred_entry = &self.tables[pred.table].entries[pred.entry_index];
                let pred_res =
                    pred_entry.get_prediction(self.config.tables[pred.table].counter_width);
                assert!(pred_res == predict_direction);
                if let Some(altpred) = m.altpred {
                    let altpred_res = match altpred {
                        TageMatchInner::Base(altpred) => {
                            let altpred_entry = &self.base_table.entries[altpred.entry_index];
                            altpred_entry.get_prediction(self.config.base_table.counter_width)
                        }
                        TageMatchInner::NonBase(altpred) => {
                            let altpred_entry =
                                &self.tables[altpred.table].entries[altpred.entry_index];
                            altpred_entry
                                .get_prediction(self.config.tables[altpred.table].counter_width)
                        }
                    };

                    if pred_res != altpred_res {
                        // update useful counter
                        if pred_res == resolve_direction {
                            // correct, increment useful
                            self.tables[pred.table].entries[pred.entry_index].increment_useful();
                        } else {
                            // incorrect, decrement useful
                            self.tables[pred.table].entries[pred.entry_index].decrement_useful();
                        }
                    }
                }

                if resolve_direction == predict_direction {
                    // correct prediction
                    if predict_direction {
                        // increment counter
                        self.tables[pred.table].entries[pred.entry_index]
                            .increment_counter(self.config.tables[pred.table].counter_width);
                    } else {
                        // decrement counter
                        self.tables[pred.table].entries[pred.entry_index].decrement_counter();
                    }
                } else {
                    // incorrect prediction
                    if predict_direction {
                        // decrement counter
                        self.tables[pred.table].entries[pred.entry_index].decrement_counter();
                    } else {
                        // increment counter
                        self.tables[pred.table].entries[pred.entry_index]
                            .increment_counter(self.config.tables[pred.table].counter_width);
                    }
                }
            }

            // wrong prediction
            if resolve_direction != predict_direction {
                // allocate in a table with longer history
                let mut allocated = false;
                for i in min_table..self.config.tables.len() {
                    if self.tables[i].allocate(
                        pc,
                        &self.history_registers,
                        resolve_direction,
                        self.config.tables[i].counter_width,
                    ) {
                        allocated = true;
                        break;
                    }
                }

                // allocation failed: decrement useful counters
                if !allocated {
                    for i in min_table..self.config.tables.len() {
                        self.tables[i].decrement_useful(pc, &self.history_registers);
                    }
                }
            }

            // update base table
            let entry_index = self.base_table.get_index(pc, &self.history_registers);
            if resolve_direction {
                self.base_table.entries[entry_index]
                    .increment_counter(self.config.base_table.counter_width);
            } else {
                self.base_table.entries[entry_index].decrement_counter();
            }
        }

        // update history registers
        if resolve_direction {
            for hr in &mut self.history_registers {
                hr.update(pc, branch_target);
            }
        }
    }

    fn update_others(
        &mut self,
        pc: u64,
        _branch_type: BranchType,
        branch_taken: bool,
        branch_target: u64,
    ) {
        // update history register
        if branch_taken {
            for hr in &mut self.history_registers {
                hr.update(pc, branch_target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        BranchType, ConditionalBranchPredictor, Tage, TageBaseTableConfig, TageConfig,
        TageHistoryRegisterConfig, TagePHRConfig, TagePHRXorConfig, TageTableConfig, TageXorConfig,
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
                base_table: TageBaseTableConfig {
                    index_bits: vec![vec![TageXorConfig::PC(0)]],
                    counter_width: 2
                },
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

    #[test]
    fn test_simple() {
        let mut tage = Tage::new("configs/firestorm.toml").unwrap();
        let mut correct = 0;
        let count = 1000;
        // branch 1: branch from 0x4 to 0x0 if i % 3 == 0
        // branch 2: branch from 0x8 to 0x0
        for i in 0..count {
            // branch 1
            let resolve_direction = if i % 3 == 0 { true } else { false };
            let predict_direction = tage.predict(0x4, resolve_direction);
            if resolve_direction == predict_direction {
                correct += 1;
            }
            tage.update(
                0x4,
                BranchType::ConditionalDirectJump,
                resolve_direction,
                predict_direction,
                0x0,
            );

            // branch 2
            if !resolve_direction {
                tage.update_others(0x8, BranchType::DirectJump, true, 0x0);
            }
        }
        assert!(correct >= 990, "{}/{}", correct, count);
    }
}
