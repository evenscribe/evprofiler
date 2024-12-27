mod encode;
pub mod executableinfo;
pub mod schema;
pub mod utils;

use crate::metapb::{Function, Mapping};
use datafusion::arrow::array::RecordBatch;
pub use encode::PprofLocations;
use serde::{Deserialize, Serialize};

pub struct Profile {
    pub meta: Meta,
    pub samples: Vec<RecordBatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationLine {
    pub line: i64,
    pub function: Option<Function>,
}

impl LocationLine {
    pub fn decode(decoded: &[u8]) -> anyhow::Result<LocationLine> {
        Ok(bincode::deserialize(decoded)?)
    }

    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Location {
    pub id: String,
    pub address: u64,
    pub is_folded: bool,
    pub mapping: Option<Mapping>,
    pub lines: Vec<LocationLine>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValueType {
    pub type_: String,
    pub unit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meta {
    pub name: String,
    pub period_type: ValueType,
    pub sample_type: ValueType,
    pub timestamp: i64,
    pub duration: i64,
    pub period: i64,
}
