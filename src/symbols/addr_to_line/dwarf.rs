use crate::symbolizer::normalize::NormalizedAddress;
use crate::{metapb, profile, symbolizer::ElfDebugInfo, symbols::Demangler};
use addr2line::LookupResult;
use object::{Object, ObjectSection};
use std::borrow;
use tonic::Status;

pub struct DwarfLiner<'data> {
    elfdbginfo: &'data ElfDebugInfo<'data>,
    demangler: &'data Demangler,
    endian: gimli::RunTimeEndian,
}

impl<'data> DwarfLiner<'data> {
    pub fn try_new(
        elfdbginfo: &'data ElfDebugInfo,
        demangler: &'data Demangler,
    ) -> anyhow::Result<Self> {
        let endian = if elfdbginfo.e.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        Ok(Self {
            elfdbginfo,
            demangler,
            endian,
        })
    }

    pub fn pc_to_lines(
        &self,
        addr: NormalizedAddress,
    ) -> anyhow::Result<Vec<profile::LocationLine>> {
        self.source_lines(addr.0)
    }

    fn source_lines(&self, addr: u64) -> anyhow::Result<Vec<profile::LocationLine>> {
        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> anyhow::Result<borrow::Cow<[u8]>> {
            Ok(match self.elfdbginfo.e.section_by_name(id.name()) {
                Some(section) => section.uncompressed_data()?,
                None => borrow::Cow::Borrowed(&[]),
            })
        };

        // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
        let borrow_section =
            |section| gimli::EndianSlice::new(borrow::Cow::as_ref(section), self.endian);

        // Load all of the sections.
        let dwarf_sections = gimli::DwarfSections::load(&load_section)?;

        // Create `EndianSlice`s for all of the sections.
        let dwarf = dwarf_sections.borrow(borrow_section);

        // Constructing a Context is somewhat costly, so users should aim to reuse Contexts when performing lookups for many addresses in the same executable.
        let c = addr2line::Context::from_dwarf(dwarf)?;

        let mut lines = vec![];
        let frames = c.find_frames(addr);

        let mut result = loop {
            match frames {
                LookupResult::Output(result) => break result,
                LookupResult::Load {
                    load: _,
                    continuation: _,
                } => {}
            }
        }?;

        loop {
            let frame = match result.next()? {
                Some(frame) => frame,
                None => break,
            };

            let function = match frame.function {
                Some(function) => function,
                None => continue,
            };

            let location = match frame.location {
                Some(location) => location,
                None => continue,
            };

            let start_line = match location.line {
                Some(line) => line as i64,
                None => continue,
            };

            let file = match location.file {
                Some(file) => file,
                None => continue,
            };

            let name = match function.raw_name() {
                Ok(name) => name,
                Err(_) => continue,
            };

            let func = self.demangler.demangle(&metapb::Function {
                id: String::default(),
                start_line,
                name: String::default(),
                system_name: name.into(),
                filename: file.to_owned(),
                name_string_index: 0,
                system_name_string_index: 0,
                filename_string_index: 0,
            });

            lines.push(profile::LocationLine {
                line: start_line,
                function: Some(func),
            });
        }

        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_cpp_symbolizer() {
        let path =
            PathBuf::from("src/symbols/addr_to_line/testdata/basic-cpp-no-fp-with-debuginfo");
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(e) => panic!("Failed to read file: {:?}", e),
        };
        let elfdbginfo = ElfDebugInfo {
            target_path: path,
            e: object::File::parse(&*data).unwrap(),
            quality: None,
        };
        let demangler = Demangler::new(false);
        let d = DwarfLiner::try_new(&elfdbginfo, &demangler).unwrap();
        let _ = d
            .pc_to_lines(NormalizedAddress(0x0000000000401156))
            .unwrap();
    }

    #[test]
    fn test_go_symbolizer() {
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
        let demangler = Demangler::new(false);
        let d = DwarfLiner::try_new(&elfdbginfo, &demangler).unwrap();
        let _ = d
            .pc_to_lines(NormalizedAddress(0x0000000000041290))
            .unwrap();
    }
}
