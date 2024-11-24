use object::{elf::SHT_SYMTAB, File, Object, ObjectSection, SectionKind};

pub fn has_symtab(e: &File<'_>) -> bool {
    for section in e.sections() {
        if section.kind() == SectionKind::Elf(SHT_SYMTAB) {
            return true;
        }
    }
    false
}
