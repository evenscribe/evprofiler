use super::NormalizedProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Series {
    pub(crate) labels: HashMap<String, String>,
    pub(crate) samples: Vec<Vec<NormalizedProfile>>,
}
