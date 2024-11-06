use super::NormalizedProfile;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Series {
    pub(crate) labels: HashMap<String, String>,
    pub(crate) samples: Vec<Vec<NormalizedProfile>>,
}
