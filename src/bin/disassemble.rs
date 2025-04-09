use capstone::prelude::*;
use clap::Parser;
use object::{Object, ObjectSection, SectionKind};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to ELF file
    elf: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()?;

    let binary_data = std::fs::read(args.elf)?;
    let file = object::File::parse(&*binary_data)?;
    for section in file.sections() {
        if section.kind() == SectionKind::Text {
            let content = section.data()?;
            let insns = cs.disasm_all(content, section.address())?;
            for insn in insns.as_ref() {
                println!("{}", insn);
            }
        }
    }
    Ok(())
}
