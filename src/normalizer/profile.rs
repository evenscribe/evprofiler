use super::NormalizedSample;
use crate::profile::Meta;

#[derive(Debug)]
pub struct NormalizedProfile {
    pub(crate) samples: Vec<NormalizedSample>,
    pub(crate) meta: Meta,
}

impl NormalizedProfile {
    pub fn new(samples: Vec<NormalizedSample>, meta: Meta) -> Self {
        Self { samples, meta }
    }
}
