use object::{File, Object};

pub fn has_dynsym(e: &File<'_>) -> bool {
    e.dynamic_symbol_table().is_some()
}
