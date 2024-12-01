use object::{File, Object};

pub fn has_dwarf(e: &File<'_>) -> bool {
    e.has_debug_symbols()
}
