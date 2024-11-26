pub mod dwarf;
mod symbol;
// pub mod go;

use super::Demangler;
use crate::symbolizer::ElfDebugInfo;
pub(crate) use dwarf::DwarfLiner;
use tonic::Status;
// mod symbol;

pub fn dwarf<'data>(
    dbg: &'data ElfDebugInfo,
    demangler: &'data Demangler,
) -> Result<DwarfLiner<'data>, Status> {
    DwarfLiner::try_new(dbg, demangler)
}
