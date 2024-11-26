use super::{normalize::NormalizedAddress, ElfDebugInfo, SymbolizerCache};
use crate::symbols::addr_to_line::{self, DwarfLiner};
use crate::{profile::LocationLine, symbols::Demangler};
use tonic::Status;

pub enum LinerKind<'data> {
    Dwarf(DwarfLiner<'data>),
    // Go,
    Symbol,
}

pub struct Liner<'data> {
    pub l: Option<LinerKind<'data>>,
    build_id: &'data str,
    elfdbginfo: &'data ElfDebugInfo<'data>,
    cache: &'data SymbolizerCache,
    demangler: &'data Demangler,
}

impl LinerKind<'_> {
    pub fn pc_to_lines(&self, pc: NormalizedAddress) -> Result<Vec<LocationLine>, Status> {
        match self {
            LinerKind::Dwarf(l) => l.pc_to_lines(pc),
            // LinerKind::Go => todo!(),
            LinerKind::Symbol => todo!(),
        }
    }
}

impl<'data> Liner<'data> {
    pub fn new(
        build_id: &'data str,
        dbginfo: &'data ElfDebugInfo,
        cache: &'data SymbolizerCache,
        demangler: &'data Demangler,
    ) -> Self {
        Self {
            build_id,
            l: None,
            elfdbginfo: dbginfo,
            cache,
            demangler,
        }
    }

    pub fn pc_to_lines(&mut self, pc: NormalizedAddress) -> Result<Vec<LocationLine>, Status> {
        // Cache lookup
        match self.cache.get(self.build_id, &pc) {
            Ok(ll) => {
                if let Some(ll) = ll {
                    return Ok(ll);
                }
            }
            Err(e) => return Err(e),
        };

        // Lazy initialization of `l`
        if self.l.is_none() {
            let new_liner = self.construct_liner()?;
            self.l = Some(new_liner);
        }

        let liner = self.l.as_ref().unwrap();
        let ll = liner.pc_to_lines(pc)?;

        // Cache the result
        let () = self.cache.set(self.build_id, &pc, ll.clone())?;
        Ok(ll)
    }

    fn construct_liner(&self) -> Result<LinerKind<'data>, Status> {
        let quality = match self.elfdbginfo.quality {
            Some(q) => q,
            None => return Err(Status::internal("No debuginfo quality found")),
        };

        if quality.has_dwarf {
            Ok(LinerKind::Dwarf(addr_to_line::dwarf(
                self.elfdbginfo,
                self.demangler,
            )?))
        }
        // else if quality.has_go_pclntab {
        // Ok(addr_to_line::go(self.elfdbginfo, self.demangler)?)
        // Ok(LinerKind::Go)
        // }
        else if quality.has_symtab || quality.has_dynsym {
            // Ok(addr_to_line::symbols(self.elfdbginfo, self.demangler)?)
            Ok(LinerKind::Symbol)
        } else {
            Err(Status::not_found("Check debuginfo quality."))
        }
    }
}
