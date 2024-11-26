use crate::symbolizer::{normalize::NormalizedAddress, ElfDebugInfo};
use object::{Object, ObjectSection, ObjectSegment, ObjectSymbol};
use tonic::Status;

pub struct GoLiner<'data> {
    elfdbginfo: &'data ElfDebugInfo<'data>,
}

impl<'data> GoLiner<'data> {
    pub fn try_new(elfdbginfo: &'data ElfDebugInfo) -> Result<Self, Status> {
        Ok(Self { elfdbginfo })
    }

    pub fn pc_to_lines(&self, pc: NormalizedAddress) -> Result<(), Status> {
        let mut text_start = 0_u64;
        if let Some(section) = self.elfdbginfo.e.section_by_name(".text") {
            text_start = section.address();
        }

        let symtab = match self.elfdbginfo.e.section_by_name(".gosymtab") {
            Some(section) => section.uncompressed_data(),
            None => {
                let section = match self.elfdbginfo.e.section_by_name(".data.rel.ro.gosymtab") {
                    Some(section) => section.uncompressed_data(),
                    None => Self::symbol_data(self.elfdbginfo, "runtime.symtab", "runtime.esymtab"),
                };
                section
            }
        };

        let pclntab = match self.elfdbginfo.e.section_by_name(".gopclntab") {
            Some(section) => section.uncompressed_data(),
            None => {
                let section = match self.elfdbginfo.e.section_by_name(".data.rel.ro.gopclntab") {
                    Some(section) => section.uncompressed_data(),
                    None => {
                        Self::symbol_data(self.elfdbginfo, "runtime.pclntab", "runtime.epclntab")
                    }
                };
                section
            }
        };

        if let Some(a) = Self::runtime_text_addr(self.elfdbginfo) {
            text_start = a;
        };

        Ok(())
    }

    fn symbol_data(
        elfdbginfo: &ElfDebugInfo<'data>,
        start: &str,
        end: &str,
    ) -> Result<std::borrow::Cow<'data, [u8]>, object::Error> {
        let symbols = elfdbginfo.e.symbols();
        let (addr, eaddr) = {
            let mut addr = 0_u64;
            let mut eaddr = 0_u64;

            for symbol in symbols {
                let name = match symbol.name() {
                    Ok(name) => name,
                    Err(_) => continue,
                };
                if name.eq(start) {
                    addr = symbol.address();
                } else if name.eq(end) {
                    eaddr = symbol.address();
                }

                if addr != 0 && eaddr != 0 {
                    break;
                }
            }

            (addr, eaddr)
        };

        let size = eaddr - addr;
        let mut data = vec![0; size as usize];

        for prog in elfdbginfo.e.segments() {
            if prog.address() <= addr && addr + size - 1 <= prog.address() + prog.size() - 1 {
                let segment_data = prog.data()?;
                let start_offset = (addr - prog.address()) as usize;
                let end_offset = start_offset + size as usize;

                if end_offset <= segment_data.len() {
                    data.copy_from_slice(&segment_data[start_offset..end_offset]);
                }
            }
        }

        Ok(data.into())
    }

    fn runtime_text_addr(elfdbginfo: &ElfDebugInfo<'data>) -> Option<u64> {
        let symbols = elfdbginfo.e.symbols();

        for symbol in symbols {
            if let Ok(name) = symbol.name() {
                if name.eq("runtime.text") {
                    return Some(symbol.address());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbolizer::normalize::NormalizedAddress;
    use std::path::PathBuf;

    #[test]
    fn test_pc_to_lines() {
        let path = PathBuf::from("src/symbols/addr_to_line/testdata/basic-go-with-debuginfo");
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(e) => panic!("Failed to read file: {:?}", e),
        };
        let elfdbginfo = ElfDebugInfo {
            target_path: path,
            e: object::File::parse(&*data).unwrap(),
            quality: None,
        };
        let d = GoLiner::try_new(&elfdbginfo).unwrap();
        let _ = d
            .pc_to_lines(NormalizedAddress(0x0000000000041290))
            .unwrap();
    }
}
