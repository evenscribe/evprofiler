use std::collections::HashMap;

#[derive(Debug)]
pub struct NormalizedSample {
    pub(crate) locations: Vec<Vec<u8>>,
    pub(crate) value: i64,
    pub(crate) diff_value: i64,
    pub(crate) label: HashMap<String, String>,
    pub(crate) num_label: HashMap<String, i64>,
}
