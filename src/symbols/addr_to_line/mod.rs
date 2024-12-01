pub mod dwarf;
mod symbol;
// pub mod go;

use super::Demangler;
use crate::symbolizer::ElfDebugInfo;
pub(crate) use dwarf::DwarfLiner;
pub(crate) use symbol::SymbolLiner;

pub fn dwarf<'data>(
    dbg: &'data ElfDebugInfo,
    demangler: &'data Demangler,
) -> anyhow::Result<DwarfLiner<'data>> {
    DwarfLiner::try_new(dbg, demangler)
}

pub fn symbol<'data>(
    dbg: &'data ElfDebugInfo,
    filename: &str,
    demangler: &'data Demangler,
) -> anyhow::Result<symbol::SymbolLiner<'data>> {
    symbol::SymbolLiner::try_new(dbg, filename, demangler)
}
