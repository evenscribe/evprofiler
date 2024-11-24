use object::{elf::SHT_DYNSYM, File, Object, ObjectSection, SectionKind};

pub fn has_dynsym(e: &File<'_>) -> bool {
    for section in e.sections() {
        if section.kind() == SectionKind::Elf(SHT_DYNSYM) {
            return true;
        }
    }
    false
}
