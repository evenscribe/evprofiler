use super::{normalize::NormalizedAddress, ElfDebugInfo, SymbolizerCache};
use crate::symbols::addr_to_line;
use crate::{profile::LocationLine, symbols::Demangler};
use tonic::Status;

pub trait LinerTrait {
    fn pc_to_lines(&self, addr: NormalizedAddress) -> Result<Vec<LocationLine>, Status>;
}

pub struct Liner<'a, T> {
    pub l: Option<T>,
    build_id: &'a str,
    elfdbginfo: &'a ElfDebugInfo<'a>,
    cache: &'a SymbolizerCache,
    demangler: &'a Demangler,
}

impl<'a, T> Liner<'a, T>
where
    T: LinerTrait,
{
    pub fn new(
        build_id: &'a str,
        dbginfo: &'a ElfDebugInfo,
        cache: &'a SymbolizerCache,
        demangler: &'a Demangler,
    ) -> Self {
        Self {
            build_id,
            l: None,
            elfdbginfo: dbginfo,
            cache,
            demangler,
        }
    }

    fn pc_to_lines(&mut self, pc: NormalizedAddress) -> Result<Vec<LocationLine>, Status> {
        let _ = match self.cache.get(self.build_id, &pc) {
            Ok(ll) => match ll {
                Some(ll) => return Ok(ll),
                _ => {}
            },
            Err(e) => return Err(e),
        };

        if self.l.is_none() {
            self.l = Some(self.construct_liner()?);
        }

        let l = self.l.as_ref().unwrap();
        let ll = l.pc_to_lines(pc)?;
        let () = self.cache.set(self.build_id, &pc, ll.clone())?;

        Ok(ll)
    }

    fn construct_liner(&self) -> Result<T, Status> {
        let quality = match self.elfdbginfo.quality {
            Some(q) => q,
            None => return Err(Status::internal("No debuginfo quality found")),
        };

        if quality.has_dwarf {
            Ok(addr_to_line::dwarf(self.elfdbginfo, self.demangler)?)
        } else if quality.has_go_pclntab {
            Ok(addr_to_line::go(self.elfdbginfo, self.demangler)?)
        } else if quality.has_symtab || quality.has_dynsym {
            Ok(addr_to_line::symbols(self.elfdbginfo, self.demangler)?)
        } else {
            Err(Status::not_found("Check debuginfo quality."))
        }
    }
}
