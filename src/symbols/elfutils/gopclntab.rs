use object::{File, Object};

pub fn has_go_pcln_tab(e: &File<'_>) -> bool {
    if let Some(_) = e.section_by_name(".gopclntab") {
        return true;
    }
    return false;
}
