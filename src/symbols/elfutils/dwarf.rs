use object::{elf::SHT_PROGBITS, File, Object, ObjectSection, SectionKind};

const PREFIXES: [&str; 3] = [".debug_", ".zdebug_", "__debug_"];

/// There are many DWARf sections, but these are the ones
/// the debug/dwarf package started with "abbrev", "info", "str", "line", "ranges".
///
/// Possible candidates for future: "loc", "loclists", "rnglists"
const ELF_SECTIONS: [&str; 5] = ["abbrev", "info", "str", "line", "ranges"];

/// has_dwarf reports whether the specified executable or library file contains DWARF debug information.
pub fn has_dwarf(e: &File<'_>) -> bool {
    search_dwarf_sections(e)
}

fn index_after_suffix(name: &str) -> usize {
    for prefix in PREFIXES {
        if name.starts_with(prefix) {
            return name.len() - prefix.len();
        }
    }
    0
}

fn search_dwarf_sections(e: &File<'_>) -> bool {
    for section in e.sections() {
        let name = section.name().unwrap();
        let suffix = index_after_suffix(name);
        let name = &&name[suffix..];

        if suffix >= name.len() && !ELF_SECTIONS.contains(name) {
            continue;
        }

        if section.kind() != SectionKind::Elf(SHT_PROGBITS) {
            continue;
        }

        return true;
    }

    return false;
}
