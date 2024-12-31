use crate::{
    metapb::Function,
    profile,
    symbolizer::{normalize::NormalizedAddress, ElfDebugInfo},
    symbols::Demangler,
};
use anyhow::bail;
use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};

#[derive(Clone, Debug)]
struct SymbolInfo {
    address: u64,
    name: String,
}

pub struct SymbolLiner<'data> {
    symbols: Vec<SymbolInfo>,
    // file_name: String,
    demangler: &'data Demangler,
}

impl<'data> SymbolLiner<'data> {
    pub fn try_new(
        elfdbginfo: &'data ElfDebugInfo,
        _: &str,
        demangler: &'data Demangler,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            // file_name: filename.to_string(),
            symbols: Self::symtab(elfdbginfo),
            demangler,
        })
    }

    pub fn pc_to_lines(&self, pc: NormalizedAddress) -> anyhow::Result<Vec<profile::LocationLine>> {
        self.source_lines(pc.0)
    }

    /// symtab returns symbols from the symbol table extracted from the ELF file.
    /// The symbols are sorted by their memory addresses in ascending order
    /// to facilitate searching.
    fn symtab(elfdbginfo: &'data ElfDebugInfo) -> Vec<SymbolInfo> {
        let mut symbols: Vec<SymbolInfo> = Vec::new();

        for symbol in elfdbginfo.e.symbols() {
            if let Ok(name) = symbol.name() {
                symbols.push(SymbolInfo {
                    address: symbol.address(),
                    name: name.to_string(),
                });
            }
        }

        for symbol in elfdbginfo.e.dynamic_symbols() {
            if let Ok(name) = symbol.name() {
                symbols.push(SymbolInfo {
                    address: symbol.address(),
                    name: name.to_string(),
                });
            }
        }

        if let Some(plt_section) = elfdbginfo.e.section_by_name(".plt") {
            let relocations = plt_section.relocations();
            for (offset, reloc) in relocations {
                if let RelocationTarget::Symbol(symbol_index) = reloc.target() {
                    if let Ok(symbol) = elfdbginfo.e.symbol_by_index(symbol_index) {
                        if let Ok(name) = symbol.name() {
                            symbols.push(SymbolInfo {
                                address: offset,
                                name: format!("{}@plt", name),
                            });
                        }
                    }
                }
            }
        }

        // Sort symbols by address
        symbols.sort_by_key(|s| s.address);

        symbols
    }

    fn source_lines(&self, pc: u64) -> anyhow::Result<Vec<profile::LocationLine>> {
        let closest_symbol = match self.find_closest_symbol(pc) {
            Some(s) => s,
            None => {
                bail!("No symbol found for the given address");
            }
        };

        let mut was_suffixed = false;
        let symbol = match closest_symbol.strip_suffix("@plt") {
            Some(s) => {
                was_suffixed = true;
                s
            }
            None => &closest_symbol,
        };

        let mut func = self.demangler.demangle(&Function {
            system_name: symbol.into(),
            filename: "?".into(),
            ..Default::default()
        });

        if was_suffixed {
            func.name = format!("{}@plt", func.name);
        }

        Ok(vec![profile::LocationLine {
            line: 0,
            function: Some(func),
        }])
    }

    fn find_closest_symbol(&self, pc: u64) -> Option<String> {
        // Binary search to find the right position
        match self.symbols.binary_search_by_key(&pc, |s| s.address) {
            Ok(index) => Some(self.symbols[index].name.clone()),
            Err(0) => None,
            Err(index) => Some(self.symbols[index - 1].name.clone()),
        }
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
        let l = SymbolLiner::try_new(&elfdbginfo, "basic-cpp-no-fp", &demangler).unwrap();
        let _ = l
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
        let l = SymbolLiner::try_new(&elfdbginfo, "basic-cpp-no-fp", &demangler).unwrap();
        let x = l
            .pc_to_lines(NormalizedAddress(0x0000000000041290))
            .unwrap();
    }
}
