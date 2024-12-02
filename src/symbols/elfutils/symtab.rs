use object::{File, Object};

pub fn has_symtab(e: &File<'_>) -> bool {
    e.symbol_table().is_some()
}
