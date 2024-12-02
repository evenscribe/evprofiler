use object::File;

pub fn has_go_pcln_tab(_: &File<'_>) -> bool {
    // e.section_by_name(".gopclntab").is_some()
    false
}
