use object::{File, Object};

pub fn has_symtab(e: &File<'_>) -> bool {
    if let Some(_) = e.symbol_table() {
        return true;
    }
    false
}
