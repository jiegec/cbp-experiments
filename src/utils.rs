use capstone::prelude::*;
use object::{Object, ObjectSection, SectionKind};
use std::collections::HashMap;

pub fn get_tqdm_style() -> indicatif::ProgressStyle {
    indicatif::ProgressStyle::with_template(
            "{percent:>3}% |{wide_bar}| {pos}/{len} [{elapsed_precise}<{eta_precise}, {custom_per_sec}]",
        )
        .unwrap()
        .with_key(
            "custom_per_sec",
            Box::new(|s: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.2} it/s", s.per_sec()).unwrap()),
        ).progress_chars("██ ")
}

// create a mapping from instruction address to instruction index for instruction counting
pub fn create_inst_index_mapping<P: AsRef<std::path::Path>>(
    elf: P,
) -> anyhow::Result<HashMap<u64, u64>> {
    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let mut mapping: HashMap<u64, u64> = HashMap::new();
    let binary_data = std::fs::read(elf)?;
    let file = object::File::parse(&*binary_data)?;

    let mut i = 0;
    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                assert_eq!(mapping.insert(insn.address(), i), None);
                i += 1;
            }
        }
    }
    Ok(mapping)
}

pub fn get_inst_index(mapping: &HashMap<u64, u64>, addr: u64) -> u64 {
    match mapping.get(&addr) {
        Some(index) => {
            // found
            *index
        }
        None => {
            // for vdso: map them to a very large number
            // so that we ignore instructions that reside in vdso
            assert!(addr >= 0x7f000000000);
            0xffffffff
        }
    }
}
