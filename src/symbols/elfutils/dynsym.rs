use object::{File, Object};

pub fn has_dynsym(e: &File<'_>) -> bool {
    if let Some(_) = e.dynamic_symbol_table() {
        return true;
    }
    false
}
