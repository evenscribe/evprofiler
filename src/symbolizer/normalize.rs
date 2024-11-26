use tonic::Status;

use crate::profile::executableinfo::{ExecutableInfo, Mapping};

#[derive(Debug, Clone, Copy)]
pub struct NormalizedAddress(pub(crate) u64);

impl NormalizedAddress {
    pub(crate) fn try_new(addr: u64, ei: &ExecutableInfo, m: &Mapping) -> Result<Self, Status> {
        let base = calculate_base(addr, ei, m)?;
        return Ok(NormalizedAddress(addr - base));
    }
}

fn calculate_base(addr: u64, ei: &ExecutableInfo, m: &Mapping) -> Result<u64, Status> {
    let h = ei.find_program_header(m, addr)?;

    let h = match h {
        Some(h) => h,
        None => return Ok(0),
    };

    if m.start == 0 && m.offset == 0 && (m.end == !(0 as u64) || m.end == 0) {
        return Ok(0);
    }

    match ei.elf_type {
        object::ObjectKind::Executable => Ok(m.start - m.offset + h.offset - h.vaddr),
        object::ObjectKind::Relocatable => {
            if m.offset != 0 {
                return Err(Status::invalid_argument(
                    "don't know how to handle mapping.Offset",
                ));
            }
            Ok(h.vaddr - h.offset + m.start)
        }
        object::ObjectKind::Dynamic => Ok(m.start - m.offset + h.offset - h.vaddr),
        _ => Err(Status::internal(format!(
            "don't know how to handle FileHeader.Type {:?}",
            ei.elf_type
        ))),
    }
}
